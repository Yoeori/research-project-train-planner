pub mod types;

use std::{collections::{HashMap, HashSet}, error::Error, fmt::Debug, io::{self, Cursor, Read}, iter::{FromIterator, Peekable}};

use chrono::{Duration, NaiveDate};
use smol_str::SmolStr;
use zip::{ZipArchive, result::ZipError};
use lazy_static::lazy_static;
use regex::Regex;
use encoding_rs::mem;
use diesel::prelude::*;

use self::types::{IFF, Station, Service, Stop};
use crate::database::types::ServiceStopType;

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
                }
            )
        }).collect();

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
            
            // while let Some(line) = service.next_if(|line| line.chars().next().unwrap() != '#') (currently unstable, same as below)
            while service.peek().is_some() && service.peek().unwrap().len() != 0 && service.peek().unwrap().chars().next().unwrap() != '#' {
                let line = service.next().unwrap();

                match line.chars().next().unwrap() {
                    '%' => {
                        // Skip for now, let's figure this out later
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
    use crate::database::schema::stations;
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

    use crate::database::schema::service_stops;
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
    for i in (0..services.len()).step_by(1000) {
        diesel::replace_into(service_stops::table).values(&stops[i..(i+1000).min(services.len())]).execute(&connection)?;
    }

    Ok(iff)
}