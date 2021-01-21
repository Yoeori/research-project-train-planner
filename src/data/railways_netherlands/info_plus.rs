use std::{collections::{BTreeSet, HashMap}, error::Error, io::Read, thread::JoinHandle};

use chrono::NaiveDate;
use diesel::{dsl::max, prelude::*};
use quick_xml::de::from_str;

use crate::{data::zeromq, types::{TripUpdate, Connection}};
use crate::database;

use super::{iff, rit_message_types::RITMessage};

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

pub fn read_dvs_to_updates(date: &NaiveDate) -> Result<Vec<TripUpdate>, Box<dyn Error>> {

    // Get timetable for given data
    let timetable = iff::get_timetable_for_day(&date)?;

    // We create a connections list using a Binary Tree, to make sure that the connections are always ordered.
    // This list should always contain the 'newest' known timetable
    let mut trips: HashMap<usize, BTreeSet<Connection>> = timetable.trips.iter()
        .map(|trip| (trip.identifier, trip.connections.clone().into_iter().collect())).collect();
    let stops: HashMap<String, usize> = timetable.stops.iter().map(|(stop_id, stop)| (stop.to_string(), *stop_id)).collect();

    let mut updates = vec![];

    // Query through all messages
    use database::schema::dvs_messages::dsl::*;
    #[derive(Debug, Queryable)]
    struct MessageQuery { message: Option<String>, envelope: Option<String> }

    const STEP: usize = 5000;

    let conn = database::establish_connection();
    let message_count = dvs_messages.select(max(id)).first::<Option<i32>>(&conn)?.unwrap() as usize;

    for i in (0..message_count).step_by(STEP) {
        let messages: Vec<MessageQuery> = dvs_messages.select((message, envelope))
            .filter(id.ge(i as i32))
            .filter(id.lt((i + STEP) as i32))
            // For now we filter on one type of message, later on this could be expended to take a look at DVS info
            // DVS info should not contain more information but might return departure information a bit earlier
            .filter(envelope.eq("/RIG/InfoPlusRITInterface2")) 
            .order(id.asc())
            .load::<MessageQuery>(&conn)?;

        for msg in messages {
            match msg.envelope.as_deref() {
                Some("/RIG/InfoPlusRITInterface2") => {

                    // We read the message to a RIT message type using Serde
                    let rit_message: RITMessage = from_str(&msg.message.as_deref().unwrap())?;

                    if &rit_message.message.rit.date != date {
                        continue;
                    }
                    
                    // Go through each seperate trip in the message
                    for rit_trip in rit_message.message.rit.trip.parts.iter()
                        .map(|t| t.to_trip(&stops)).filter(|t| {
                            if t.is_none() {
                                println!("{}", &msg.message.as_deref().unwrap()); // Log trips which cannot be serialized
                            }
                            t.is_some()
                        }).map(|t| t.unwrap()) {

                        // TODO: One current bug is the fact that date+train id for RIT messages does not always correspond
                        // to the same date+train id in IFF. This is the case for some trains that are planned in a timeschedule day
                        // after midnight.

                        // Discussion: Not sure why this is done, this seems like more of a problem of different systems not corresponding
                        // to the same format making it impossible to exactly match the incoming update to the corresponding trip
                        // Possible solutions:
                        // - Rewrite IFF to timetable trips that fully take place the next day, on the next day
                        // - Match trip using original planned departure time (requires keeping an original lookup of some sort)
                        // - Acquire information for NS Reizigers to the expected way IFF and RIT should be matched
                        // The first two solutions are also not guaranteed to be a solution, more research should be done for that.
                        // Funally enough the same problem is also found on https://treinposities.nl, e.g. for the 1410

                        if let Some(trip) = trips.get_mut(&rit_trip.identifier) {
                            let rit_trip_set: BTreeSet<Connection> = rit_trip.connections.clone().into_iter().collect();

                            // We try to discover the differences between the rit_trip and the current known trip
                            let del_connections: Vec<Connection> = trip.difference(&rit_trip_set).map(|x| x.clone()).collect();
                            let new_connections: Vec<Connection> = rit_trip_set.difference(trip).map(|x| x.clone()).collect();

                            for new_connection in new_connections {
                                trip.insert(new_connection.clone());
                                updates.push(TripUpdate::AddConnection { trip: rit_trip.clone(), connection: new_connection });
                            }

                            for del_connection in del_connections {
                                trip.remove(&del_connection);
                                updates.push(TripUpdate::DeleteConnection { trip: rit_trip.clone(), connection: del_connection });
                            }

                        } else {
                            // This is a new trip, we add the trip.
                            trips.insert(rit_trip.identifier, rit_trip.connections.clone().into_iter().collect());
                            updates.push(TripUpdate::AddTrip { trip: rit_trip });
                        }
                    }
                },
                _ => {}
            }
        }

        println!("Finished: {}", i + 1000);
    }
    
    Ok(updates)
}