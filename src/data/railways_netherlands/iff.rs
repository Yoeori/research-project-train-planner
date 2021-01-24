use std::{collections::{HashMap, HashSet}, error::Error, fmt::Debug, io::{self, Cursor, Read}, iter::{FromIterator, Peekable}, ops::Range};

use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone};
use smol_str::SmolStr;
use zip::{ZipArchive, result::ZipError};
use lazy_static::lazy_static;
use regex::Regex;
use encoding_rs::mem;
use diesel::{prelude::*, sql_types::Date};
use itertools::Itertools;

use super::iff_types::{IFF, Station, Service, Stop};
use crate::{database::types::ServiceStopType, types::{Connection, Timetable, Trip}};
use crate::database::schema::service_stops;

lazy_static! {
    static ref RE_TRNSMODE: Regex = Regex::new(r"(?P<id>[A-Z]+)\s*,(?P<description>[\w \.]*[\w\.]+)\s*").unwrap();
    static ref RE_IDENT: Regex    = Regex::new(r"@(?P<company>\d{3}),(?P<valid_from>\d{8}),(?P<valid_till>\d{8}),(?P<version>\d{4}),(?P<description>[\w\-,_ ]*[\w+])").unwrap();
}

impl IFF {
    fn from_zip<R: Read+ io::Seek>(mut zip: ZipArchive<R>) -> Result<Self, Box<dyn Error>> {

        fn get_file<R: Read + io::Seek>(file: &str, z: &mut ZipArchive<R>) -> Result<String, ZipError> {
            let mut file = z.by_name(file)?;
            let mut text = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut text)?;
            Ok(mem::decode_latin1(&text).to_string())
        }

        let stations: HashMap<SmolStr, Station> = get_file("stations.dat", &mut zip)?.lines().skip(1).map(|line| {
            (
                SmolStr::new(&line[2..9].trim_end()),
                Station {
                    is_interchange_station: &line[0..1] == "1",
                    code: line[2..9].trim_end().to_string(),
                    name: line[43..].trim_end().to_string(),
                    country: line[16..20].trim().to_string(),
                    timezone: line[21..25].parse().unwrap(),
                    interchange_duration: line[10..12].parse().unwrap(),
                    lat: line[29..35].parse::<i32>().unwrap() * 10,
                    lng: line[36..42].parse::<i32>().unwrap() * 10
                }
            )
        }).collect();

        // std::process::exit(0x0);

        let trns_modes: HashMap<SmolStr, String> = RE_TRNSMODE.captures_iter(&get_file("trnsmode.dat", &mut zip)?).map(|cap| {
            (
                SmolStr::new(cap.name("id").unwrap().as_str()),
                cap.name("description").unwrap().as_str().to_string()
            )
        }).collect();

        fn parse_service<'a>(service: &mut Peekable<impl Iterator<Item=&'a str> + Debug>) -> Result<Service, Box<dyn Error>> {
            if service.peek().unwrap().is_empty() {
                Err("End of file")?
            }

            let identification: usize = service.next().unwrap()[1..].parse()?;
            let mut validity = None;

            let mut trns_modes = vec![];
            let mut attributes = vec![];
            let mut stops = vec![];
            let mut service_number = vec![];
            
            // while let Some(line) = service.next_if(|line| line.chars().next().unwrap() != '#') (currently unstable, same as below)
            while service.peek().is_some() && service.peek().unwrap().len() != 0 && service.peek().unwrap().chars().next().unwrap() != '#' {
                let line = service.next().unwrap();

                match line.chars().next().unwrap() {
                    '%' => {
                        service_number.push((
                            line[5..10].parse()?,
                            line[18..21].parse()?..line[22..25].parse()?
                        ));
                    },
                    '-' => validity = Some(line[1..6].parse()?),
                    '&' => trns_modes.push(line[1..5].trim_end().to_string()),
                    '*' => attributes.push(line[1..5].trim_end().to_string()),
                    '>' => {
                        stops.push(Stop::Departure {
                            dep_time: line[9..13].parse()?,
                            station: line[1..8].trim_end().to_string(),
                            track: None
                        });
                    },
                    '.' => stops.push(Stop::ShortIntermediate {
                            dep_time: line[9..13].parse()?,
                            station: line[1..8].trim_end().to_string(),
                            track: None
                        }),
                    ';' => stops.push(Stop::Pass {
                            station: line[1..8].trim_end().to_string()
                        }),
                    '+' => stops.push(Stop::Intermediate {
                            dep_time: line[14..18].parse()?,
                            arr_time: line[9..13].parse()?,
                            station: line[1..8].trim_end().to_string(),
                            track: None
                        }),
                    '?' => {
                        let first_track = Some(line[1..6].trim_end().to_string());
                        let sec_track = Some(line[7..12].trim_end().to_string());

                        match stops.last_mut().ok_or("Got track information before stops")? {
                            Stop::Departure { track, .. } => *track = sec_track,
                            Stop::ShortIntermediate { track, .. } => *track = sec_track,
                            Stop::Intermediate { track, .. } => *track = sec_track,
                            Stop::Arrival { track, .. } => *track = first_track,
                            Stop::Pass { .. } => Err("Tried to add track information to pass")?
                        };
                    },
                    '<' => stops.push(Stop::Arrival {
                            arr_time: line[9..13].parse()?,
                            station: line[1..8].trim_end().to_string(),
                            track: None
                        }),
                    _ => Err(format!("Unknown line in timetable: {}", line))?
                }
            }

            Ok(Service {
                identification,
                service_number,
                validity: validity.ok_or("No validity set")?,
                trns_modes,
                attributes,
                stops
            })
        }

        let timetable = get_file("timetbls.dat", &mut zip)?;
        let mut timetable_iterator = timetable.split("\r\n").skip(1).peekable();
        let mut services = HashMap::new();
        
        loop {
            let service = parse_service(&mut timetable_iterator);
            if let Ok(service) = service {
                services.insert(service.identification, service);
            } else if let Err(err) = service {
                if err.to_string() == "End of file" { // Should probably make this an error type
                    break;
                } else {
                    return Err(err);
                }
            }
        }

        // Validity
        let mut validity: HashMap<usize, HashSet<NaiveDate>> = HashMap::new();
        let validity_file = get_file("footnote.dat", &mut zip)?;
        let mut validity_iter = validity_file.lines();
        let validity_information = RE_IDENT.captures(validity_iter.next().ok_or("Footnote file empty")?).ok_or("Invalid footnote file")?;

        let validity_from = NaiveDate::parse_from_str(validity_information.name("valid_from").unwrap().as_str(), "%d%m%Y")?;
        while let Some(validity_id) = validity_iter.next() {
            validity.insert(validity_id[1..].parse::<usize>()?,
                validity_iter.next().unwrap().chars().enumerate().filter(|(_, c)| c == &'1').map(|(i, _)| validity_from + Duration::days(i as i64)).collect());
        }

        Ok(IFF {
            stations,
            trns_modes,
            services,
            validity
        })
    }
}

#[allow(dead_code)]
/// Truncates the IFF data from the database and downloads a new version which is than inserted to the database
pub async fn update_iff_database() -> Result<IFF, Box<dyn Error + 'static>> {

    // Load IFF file
    let iff = IFF::from_zip(ZipArchive::new(Cursor::new(reqwest::get("http://data.ndovloket.nl/iff/ns-latest.zip").await?.bytes().await?))?)?;

    let connection = crate::database::establish_connection();

    // Upload stations to database
    // use crate::database::schema::stations;
    diesel::replace_into(stations::table).values(Vec::from_iter(iff.stations.values())).execute(&connection)?;
    
    // Upload trns_modes to database
    use crate::database::schema::trns_modes;
    #[derive(Debug, Insertable)]
    #[table_name = "trns_modes"]
    struct TrnsModeInsertable<'a> { id: &'a str, description: &'a str }
    diesel::replace_into(trns_modes::table).values(
        iff.trns_modes.iter().map(|(id, des)| TrnsModeInsertable { id: id.as_str(), description: des }).collect::<Vec<TrnsModeInsertable>>()
    ).execute(&connection)?;

    // Validities
    use crate::database::schema::validities;
    #[derive(Debug, Insertable)]
    #[table_name = "validities"]
    struct ValidityInsertable<'a> { id: u32, date: &'a NaiveDate }

    let validities = iff.validity.iter().map(|(id, dates)| dates.iter().map(move |date| {
        ValidityInsertable { id: *id as u32, date }
    })).flatten().collect::<Vec<ValidityInsertable>>();

    for i in (0..validities.len()).step_by(1000) {
        diesel::replace_into(validities::table).values(&validities[i..(i+1000).min(validities.len())]).execute(&connection)?;
    }

    // Services
    use crate::database::schema::services;
    #[derive(Debug, Insertable)]
    #[table_name = "services"]
    struct ServiceInsertable { id: u32, validity_id: u32 }
    let services = iff.services.values().map(|service| ServiceInsertable { id: service.identification as u32, validity_id: service.validity as u32 }).collect::<Vec<ServiceInsertable>>();
    for i in (0..services.len()).step_by(1000) {
        diesel::replace_into(services::table).values(&services[i..(i+1000).min(services.len())]).execute(&connection)?;
    }

    // Service identifiers
    use crate::database::schema::service_identifier;
    #[derive(Debug, Insertable)]
    #[table_name = "service_identifier"]
    struct ServiceIdentifierInsertable { service_id: u32, identifier: u32, from_index: u16, to_index: u16 };

    let service_identifiers = iff.services.values()
        .map(|service| service.service_number.iter().map(move |id| ServiceIdentifierInsertable {
            service_id: service.identification as u32,
            identifier: id.0 as u32,
            from_index: id.1.start as u16,
            to_index: id.1.end as u16
        })).flatten().collect::<Vec<ServiceIdentifierInsertable>>();

    for i in (0..service_identifiers.len()).step_by(1000) {
        diesel::replace_into(service_identifier::table).values(&service_identifiers[i..(i+1000).min(service_identifiers.len())]).execute(&connection)?;
    }

    #[derive(Debug, Insertable)]
    #[table_name = "service_stops"]
    struct StopInsertable<'a> {
        service_id: u32,
        ordering: u32,
        type_: ServiceStopType,
        station_code: Option<&'a String>,
        arr_time: Option<u16>,
        dep_time: Option<u16>,
        platform: Option<&'a String>
    }

    fn service_stops_to_stop_insertable(service: &Service) -> Vec<StopInsertable> {
        let mut res = vec![];
        for (i, stop) in service.stops.iter().enumerate() {
            res.push(match stop {
                Stop::Departure { dep_time, station, track } => StopInsertable {
                    service_id: service.identification as u32,
                    ordering: i as u32,
                    type_: ServiceStopType::Departure,
                    station_code: Some(station),
                    arr_time: None,
                    dep_time: Some(*dep_time),
                    platform: track.as_ref()
                },
                Stop::ShortIntermediate { dep_time, station, track } => StopInsertable {
                    service_id: service.identification as u32,
                    ordering: i as u32,
                    type_: ServiceStopType::ShortIntermediate,
                    station_code: Some(station),
                    arr_time: None,
                    dep_time: Some(*dep_time),
                    platform: track.as_ref()
                },
                Stop::Intermediate { arr_time, dep_time, station, track } => StopInsertable {
                    service_id: service.identification as u32,
                    ordering: i as u32,
                    type_: ServiceStopType::Intermediate,
                    station_code: Some(station),
                    arr_time: Some(*arr_time),
                    dep_time: Some(*dep_time),
                    platform: track.as_ref()
                },
                Stop::Pass { station } => StopInsertable {
                    service_id: service.identification as u32,
                    ordering: i as u32,
                    type_: ServiceStopType::Pass,
                    station_code: Some(station),
                    arr_time: None,
                    dep_time: None,
                    platform: None
                },
                Stop::Arrival { arr_time, station, track } => StopInsertable {
                    service_id: service.identification as u32,
                    ordering: i as u32,
                    type_: ServiceStopType::Arrival,
                    station_code: Some(station),
                    arr_time: Some(*arr_time),
                    dep_time: None,
                    platform: track.as_ref()
                }
            });
        }

        res
    }

    let stops = iff.services.values().map(|service| service_stops_to_stop_insertable(service)).flatten().collect::<Vec<StopInsertable>>();
    for i in (0..stops.len()).step_by(1000) {
        diesel::replace_into(service_stops::table).values(&stops[i..(i+1000).min(stops.len())]).execute(&connection)?;
    }

    Ok(iff)
}

use crate::database::schema::stations;
#[derive(Debug, PartialEq, Eq, Hash, QueryableByName, Clone)]
#[table_name = "stations"]
struct IFFStop {
    code: String,

    // Currently the latitude and longitude ar in the rijksdriehoekscoördinatensystem, should probably be converted in to WG84
    // However RD has the added benefit of being semi-distance-accurate which means we don't need to do difficult
    //  distance calculations!
    lat: i32,
    lng: i32

    // TODO for eventually adding interchanges at stations
    // Main problem is that there can be platforms in the updates which weren't present in the original timetable
    // Which means that the stops list in the timetable need to be updated after use
    // Probably best way is to use some kind of interior mutability pattern?
    // platform: Option<String>
}

impl crate::types::Stop for IFFStop {
    fn to_string(&self) -> String {
        format!("{}", self.code)
    }

    fn distance(&self, other: &Box<dyn crate::types::Stop>) -> Option<f64> {
        let c1 = self.coords().unwrap();
        let c2 = other.coords().unwrap();

        Some(((c1.0 - c2.0).powi(2) + (c1.1 - c2.1).powi(2)).sqrt())
    }

    fn coords(&self) -> Option<(f64, f64)> {
        Some((self.lat as f64, self.lng as f64))
    }
}

pub fn get_timetable_for_day(date: &NaiveDate) -> Result<Timetable, Box<dyn Error>> {

    fn query_to_trips(connections: Vec<QueryConnection>, stops: &HashMap<&String, usize>, service_ids: &Vec<(usize, Range<usize>)>, datetime: &DateTime<Local>) -> Vec<Trip> {
        service_ids.iter().map(|(id, range)| {
            Trip {
                identifier: *id,
                connections: query_to_trip(&connections[range.clone()], &stops, datetime, *id)
            }
        }).collect()
    }

    fn query_to_trip(query_connections: &[QueryConnection], stops: &HashMap<&String, usize>, datetime: &DateTime<Local>, id: usize) -> Vec<Connection> {
        let mut connections = vec![];
        let mut prev_connection = &query_connections[0];
        for next_connection in &query_connections[1..] {

            let dep_time = prev_connection.dep_time.unwrap() as i64;
            let arr_time = if let Some(arr_time) = next_connection.arr_time {
                arr_time
            } else {
                next_connection.dep_time.unwrap()
            } as i64;

            connections.push(Connection {
                dep_stop: *stops.get(&prev_connection.station_code).unwrap(),
                arr_stop: *stops.get(&next_connection.station_code).unwrap(),
                
                dep_time: (*datetime + Duration::hours(dep_time / 100) + Duration::minutes(dep_time % 100)).timestamp() as u32,
                arr_time: (*datetime + Duration::hours(arr_time / 100) + Duration::minutes(arr_time % 100)).timestamp() as u32,

                trip_id: id
            });
            prev_connection = next_connection;
        }
        connections
    }
    
    let conn = crate::database::establish_connection();

    #[derive(Debug, Queryable)]
    struct QueryServiceIdentifier{ service_id: u32, identifier: u32, from_index: u16, to_index: u16 };

    use crate::database::schema::service_identifier::dsl::*;
    let service_ids: HashMap<usize, Vec<(usize, Range<usize>)>> = service_identifier.load::<QueryServiceIdentifier>(&conn)?
        .into_iter()
        .group_by(|item| item.service_id)
        .into_iter()
        .map(|(id, items)| (id as usize, 
            items.map(|id| (id.identifier as usize, (id.from_index as usize - 1)..(id.to_index as usize))).collect()
        ))
        .collect();

    #[derive(Debug, QueryableByName)]
    #[table_name = "service_stops"]
    struct QueryConnection {
        service_id: u32,
        type_: ServiceStopType,
        station_code: String,
        arr_time: Option<u16>,
        dep_time: Option<u16>,
        platform: Option<String>
    }


    // Get stop list
    let stops: HashMap<IFFStop, usize> = diesel::sql_query(include_str!("stop_list.sql")).load::<IFFStop>(&conn)?
        .into_iter()
        .enumerate().map(|(i, stop)| (stop, i))
        .collect();

    let stops_lookup: HashMap<&String, usize> = stops.iter().map(|(stop, id)| (&stop.code, *id)).collect();
    let datetime = Local.from_utc_date(&date).and_hms(0, 0, 0);

    let trips = diesel::sql_query(include_str!("timetable_for_day.sql"))
        .bind::<Date, _>(&date)
        .load::<QueryConnection>(&conn)?
        .into_iter()
        .group_by(|stop| stop.service_id)
        .into_iter()
        .map(|(id, connections)| query_to_trips(connections.collect(), &stops_lookup, service_ids.get(&(id as usize)).unwrap(), &datetime))
        .flatten().collect::<Vec<Trip>>();

    // Now we create a 'loopback' footpath for each station
    let mut footpaths = HashMap::new();
    for (_, &stop) in &stops {
        footpaths.insert(stop, vec![(stop, 12*60)]);
    }

    Ok(Timetable {
        trips,
        stops: stops.into_iter().map(|(stop, i)| (i, Box::new(stop) as Box<dyn crate::types::Stop>)).collect(),
        footpaths
    })
}