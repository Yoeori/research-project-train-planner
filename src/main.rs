#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

mod types;
mod benchable;
mod benchmarking;
mod algorithms;
mod data;
pub mod database;

use std::error::Error;
use benchable::Benchable;
use chrono::NaiveDate;
use clap::{App, Arg, SubCommand};

// Embeds migrations from migrations folder
embed_migrations!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + 'static>> {

    // Load .env variables
    dotenv::dotenv()?;

    // Make sure our database is up-to-date
    let connection = database::establish_connection();
    embedded_migrations::run(&connection)?;
    std::mem::drop(connection);

    // Parse command line arguments
    let app = App::new("Train planner benchmarking utilities")
        .about("App for gathering data and benchmarking data on different public transport routing algorithms")
        .author("Yoeri Otten <y.d.otten@student.utwente.nl>")
        .subcommand(SubCommand::with_name("iff").about("Download, parse and import the most up-to-date IFF file from the NDOV Loket"))
        .subcommand(SubCommand::with_name("dvs").about("Listen for DVS messages").arg(Arg::with_name("envelopes")
            .help("Which set of envelopes to run")
            .possible_values(&["all", "dvs", "rit"])
            .default_value("all")))
        .subcommand(SubCommand::with_name("bench")
            .help("Perform all benchmarks"))
        .subcommand(SubCommand::with_name("example"))
            .help("Gets route from Enschede Kennispark to Amersfoort Centraal on 2021-01-15 at 12:00")
        .subcommand(SubCommand::with_name("trip")
            .about("Looks up a specific trip on 2021-01-15")
            .arg(Arg::with_name("id").help("Train number").required(true)
        ))
        .get_matches();

    match app.subcommand() {
        ("iff", _) => {
            println!("Starting update of IFF data, this might take a while...");
            data::iff::update_iff_database().await?;
        },
        ("dvs", Some(sub_matches)) => {
            println!("Starting to listen for DVS messages...");

            let envelopes = match sub_matches.value_of("envelopes") {
                Some("all") => data::dvs::ENVELOPES_ALL,
                Some("dvs") => data::dvs::ENVELOPES_DVS,
                Some("rit") => data::dvs::ENVELOPES_RIT,
                other => panic!("Unknown command line value for envelopes: {:?}", other)
            };

            data::dvs::dvs_stream(envelopes).into_iter().for_each(|x| x.join().unwrap().unwrap());
        }
        ("bench", _) => {
            println!("Generating timetable and updates list, this might take a while...");
            let timetable = data::iff::get_timetable_for_day(NaiveDate::from_ymd(2021, 1, 15))?;
            let updates = data::dvs::read_dvs_to_updates()?;

            println!("Starting bench of static algorithms..");
            benchmarking::bench_algorithms("IFF", &timetable)?;

            println!("Starting bench of live algorithms..");
            benchmarking::bench_algorithms_live("IFF", &timetable, &updates)?;
        }
        ("trip", Some(sub_matches)) => {
            let id = sub_matches.value_of("id").unwrap().parse().unwrap();
            println!("Looking up route of service {}", id);

            let timetable = data::iff::get_timetable_for_day(NaiveDate::from_ymd(2021, 1, 15))?;
            let trip = timetable.trips.iter().find(|x| x.identifier == id).unwrap();

            for conn in &trip.connections {
                println!("{:?} at {:?} => {:?} at {:?}", timetable.stops.get(&conn.dep_stop).unwrap(), conn.dep_time, timetable.stops.get(&conn.arr_stop).unwrap(), conn.arr_time);
            }
        }
        ("example", _) => {
            // Performs an example routing with CSA Vec
            let timetable = data::iff::get_timetable_for_day(NaiveDate::from_ymd(2021, 1, 15))?;

            // Find ID's of amf and esk
            let amf = timetable.stops.iter().find(|(_, stop)| stop.to_string() == "amf").unwrap().0;
            let esk = timetable.stops.iter().find(|(_, stop)| stop.to_string() == "esk").unwrap().0;

            let alg = algorithms::csa_vec::CSAVec::new(&timetable);
            let route = alg.find_earliest_arrival(*esk, *amf, 120000).unwrap();

            for conn in &route.connections {
                println!("{:?} at {:?} => {:?} at {:?}", timetable.stops.get(&conn.dep_stop).unwrap(), conn.dep_time, timetable.stops.get(&conn.arr_stop).unwrap(), conn.arr_time);
            }
        }
        _ => {},
    }

    Ok(())
}