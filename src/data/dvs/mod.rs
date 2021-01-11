mod dvs_message_types;

use std::{io::Read, sync::{Arc, Mutex}, thread::JoinHandle};
use std::ops::Deref;

// use quick_xml::de::from_str;
use diesel::prelude::*;

use crate::data::zeromq;
use crate::database;
// use dvs_message_types::DVSMessage;

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

    let db_conn = Arc::new(Mutex::new(database::establish_connection()));

    // Spawn multiple connection to distinguish between envelopes
    envelopes.iter().map(|&env| {
        let conn = db_conn.clone();
        std::thread::spawn(move || {

            let subscription = zeromq::subscribe("tcp://pubsub.besteffort.ndovloket.nl:7664", &[env]).unwrap();
            let env = std::str::from_utf8(env).unwrap();

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
                    .values(DVSMessageInsertable { message: &message, envelope: env }).execute(conn.lock().unwrap().deref())?;

                // Interpret message
                // let dvs_message: Result<DVSMessage, _> = from_str(&message);
                // if let Err(err) = dvs_message {
                //     println!("{}", err);
                //     println!("{}", message);
                // } else if let Ok(dvs_message) = dvs_message {
                //     dbg!(dvs_message);
                // }
            }
        })
    }).collect()
    
}