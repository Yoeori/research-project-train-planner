mod dvs_message_types;
mod rit_message_types;

use std::{collections::HashMap, error::Error, io::Read, thread::JoinHandle};

use chrono::{DateTime, Datelike, Local, NaiveDate};
use diesel::{dsl::max, prelude::*};
use quick_xml::de::from_str;

use crate::{data::zeromq, types::{Trip, TripUpdate, Connection}};
use crate::database;

use self::rit_message_types::{RITLogicalPart, RITState};
use self::rit_message_types::RITMessage;

#[allow(dead_code)]
pub const ENVELOPES_ALL: &[&[u8]] = &[
    b"/RIG/InfoPlusDVSInterface4",
    b"/RIG/InfoPlusPILInterface5",
    b"/RIG/InfoPlusPISInterface5",
    b"/RIG/InfoPlusVTBLInterface5",
    b"/RIG/InfoPlusVTBSInterface5",
    b"/RIG/NStreinpositiesInterface5",
    b"/RIG/InfoPlusRITInterface2"
];

#[allow(dead_code)]
pub const ENVELOPES_DVS: &[&[u8]] = &[
    b"/RIG/InfoPlusDVSInterface4",
];

#[allow(dead_code)]
pub const ENVELOPES_RIT: &[&[u8]] = &[
    b"/RIG/InfoPlusRITInterface2"
];

#[allow(dead_code)]
pub fn dvs_stream(envelopes: &'static [&[u8]]) -> Vec<JoinHandle<Result<(), Box<diesel::result::Error>>>> {

    // Spawn multiple connection to distinguish between envelopes
    envelopes.iter().map(|&env| {
        std::thread::spawn(move || {

            let subscription = zeromq::subscribe("tcp://pubsub.besteffort.ndovloket.nl:7664", &[env]).unwrap();
            let db_conn = database::establish_connection();
            let env = std::str::from_utf8(env).unwrap();

            println!("Listening on {}", env);

            loop {
                let mut xml = zeromq::receive(&subscription).unwrap();

                // Normally XML can be used directly with from_reader, however for debugging we make a String copy.
                let mut message = String::new();
                xml.read_to_string(&mut message).unwrap();

                // Store XML in database
                use crate::database::schema::dvs_messages;
                #[derive(Debug, Insertable)]
                #[table_name = "dvs_messages"]
                struct DVSMessageInsertable<'a> { message: &'a String, envelope: &'a str }
                diesel::insert_into(dvs_messages::table)
                    .values(DVSMessageInsertable { message: &message, envelope: env }).execute(&db_conn)?;

            }
        })
    }).collect()
    
}

// TODO Currently very memory inefficient and cannot find deleted / added connections
// Should probably use a BTree in the future for automatic ordering of connections and easier removal / insertions
pub fn read_dvs_to_updates() -> Result<Vec<TripUpdate>, Box<dyn Error>> {
    // First we get a day's timetable
    let date = NaiveDate::from_ymd(2021, 1, 15);
    let timetable = crate::data::iff::get_timetable_for_day(date)?;

    let mut trip_updates: Vec<TripUpdate> = vec![];

    // We make a full copy of all trips, all changes created compared to this list
    let mut original_trips: HashMap<usize, Trip> = timetable.trips.iter().map(|trip| (trip.identifier, trip.clone())).collect();
    let mut trips = original_trips.clone();
    let stops: HashMap<String, usize> = timetable.stops.iter().map(|(stop_id, stop)| (stop.to_string(), *stop_id)).collect();

    #[derive(Debug, Queryable)]
    struct DVSMessageQuery { message: Option<String>, envelope: Option<String> }

    let conn = database::establish_connection();

    use database::schema::dvs_messages::dsl::*;
    let total_messages = dvs_messages.select(max(id)).first::<Option<i32>>(&conn)?.unwrap() as usize;

    for i in (0..total_messages).step_by(5000) {
        let messages: Vec<DVSMessageQuery> = dvs_messages.select((message, envelope))
            .filter(id.ge(i as i32))
            .filter(id.lt(i as i32 + 5000))
            .filter(envelope.eq("/RIG/InfoPlusRITInterface2"))
            .order(id.asc())
            .load::<DVSMessageQuery>(&conn)?;

        for msg in messages {
            match msg.envelope.as_deref() {
                Some("/RIG/InfoPlusDVSInterface4") => {
                    // Might be used in the future, currently irrelevant
                },
                Some("/RIG/InfoPlusRITInterface2") => {

                    // Interpret message
                    let msg = &msg.message.unwrap();
                    let dvs_message: Result<RITMessage, _> = from_str(msg);
                    if let Err(err) = dvs_message {
                        println!("{}", err);
                        println!("{}", msg);
                    } else if let Ok(rit_message) = dvs_message {
                        if rit_message.message.rit.date == date {
                            for trip in &rit_message.message.rit.trip.parts {

                                // First we check if our trip already exists, otherwise we create a 'new trip'
                                if !&trips.contains_key(&trip.trip_id) {
                                    let new_trip = new_trip_from_rit(trip, &stops);

                                    trip_updates.push(TripUpdate::AddTrip {
                                        trip: new_trip.clone()
                                    });

                                    trips.insert(trip.trip_id, new_trip.clone());
                                    original_trips.insert(trip.trip_id, new_trip);
                                }

                                // Than we lookup changes to the trip
                                let mut i = 0;
                                let cur_stops = &mut trips.get_mut(&trip.trip_id).unwrap();
                                let cur_trip = cur_stops.clone();
                                let cur_stops = &mut cur_stops.connections;

                                let mut prev_stop = &trip.stops[0];
                                for next_stop in &trip.stops[1..] {
                                    let s1 = &timetable.stops.get(&cur_stops[i].arr_stop).unwrap().to_string();
                                    let s2 = &next_stop.station.code.to_ascii_lowercase();

                                    if s1 == s2 {
                                        
                                        let conn = Connection {
                                            dep_stop: *stops.get(&prev_stop.station.code.to_lowercase()).unwrap(),
                                            arr_stop: *stops.get(&next_stop.station.code.to_lowercase()).unwrap(),

                                            dep_time: time_to_num(&prev_stop.dep_time.iter().find(|s| s.state == RITState::Current).unwrap().date),
                                            arr_time: time_to_num(&next_stop.arr_time.iter().find(|s| s.state == RITState::Current).unwrap().date)
                                        };

                                        if cur_stops[i] != conn {
                                            trip_updates.push(TripUpdate::UpdateConnection {
                                                trip: cur_trip.clone(),
                                                connection_old: cur_stops[i].clone(),
                                                connection_new: conn.clone()
                                            });

                                            cur_stops[i] = conn;
                                        }

                                        i += 1;
                                        prev_stop = next_stop;
                                    }
                                }
                            }
                        }
                    }

                }
                _ => {}
            }
        }

        println!("Finished: {}", i + 1000);
    }

    Ok(trip_updates)
}

fn new_trip_from_rit(rit: &RITLogicalPart, stops: &HashMap<String, usize>) -> Trip {
    let mut connections = vec![];

    let mut prev_stop = &rit.stops[0];
    for next_stop in &rit.stops[1..] {
        if next_stop.stopping.iter().find(|s| s.state == RITState::Planned).unwrap().stopping {
            connections.push(Connection {
                dep_stop: *stops.get(&prev_stop.station.code.to_lowercase()).unwrap(),
                arr_stop: *stops.get(&next_stop.station.code.to_lowercase()).unwrap(),

                dep_time: time_to_num(&prev_stop.dep_time.iter().find(|s| s.state == RITState::Planned).unwrap().date),
                arr_time: time_to_num(&next_stop.arr_time.iter().find(|s| s.state == RITState::Planned).unwrap().date)
            });
            prev_stop = next_stop;
        }
    }

    Trip {
        identifier: rit.trip_id,
        connections
    }
}

fn time_to_num(datetime: &DateTime<Local>) -> u32 {
    (if datetime.day() == 16 { // TODO FIX this is a hack which only works in this specific instance due to how the IFF timetable works.
        240000u32
    } else {
        0u32
    }) + datetime.time().format("%H%M%S").to_string().parse::<u32>().unwrap()
}