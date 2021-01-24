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
use benchable::{Benchable, BenchableLive};
use chrono::{Local, NaiveDate, TimeZone};
use clap::{App, Arg, SubCommand};

use data::railways_netherlands::{info_plus, iff};

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
        .subcommand(SubCommand::with_name("bench").about("Perform benchmarks with specified dataset").arg(Arg::with_name("set")
            .help("Which data set to use for benching")
            .possible_values(&["iff", "trainline"])
            .default_value("iff")))
        .subcommand(SubCommand::with_name("example"))
            .about("Gets route from Enschede Kennispark to Amersfoort Centraal on 2021-01-15 at 12:00")
        .subcommand(SubCommand::with_name("trip")
            .about("Looks up a specific trip on 2021-01-15")
            .arg(Arg::with_name("id").help("Train number").required(true)
        ))
        .get_matches();

    match app.subcommand() {
        ("iff", _) => {
            println!("Starting update of IFF data, this might take a while...");
            iff::update_iff_database().await?;
        },
        ("dvs", Some(sub_matches)) => {
            println!("Starting to listen for DVS messages...");

            let envelopes = match sub_matches.value_of("envelopes") {
                Some("all") => info_plus::ENVELOPES_ALL,
                Some("dvs") => info_plus::ENVELOPES_DVS,
                Some("rit") => info_plus::ENVELOPES_RIT,
                other => panic!("Unknown command line value for envelopes: {:?}", other)
            };

            info_plus::dvs_stream(envelopes).into_iter().for_each(|x| x.join().unwrap().unwrap());
        }
        ("bench", Some(sub_matches)) => {
            match sub_matches.value_of("set") {
                Some("iff") => {
                    println!("Generating timetable and updates list, this might take a while...");
                    let date = NaiveDate::from_ymd(2021, 1, 15);
                    let timetable = iff::get_timetable_for_day(&date)?;
                    let updates = info_plus::read_dvs_to_updates(&date)?;

                    println!("The timetable contains {} connections, stopping at {} places and contains {} updates.", 
                        &timetable.trips.iter().map(|t| t.connections.len()).sum::<usize>(),
                        &timetable.stops.len(),
                        updates.len()
                    );

                    println!("Starting bench of static algorithms..");
                    benchmarking::bench_algorithms("IFF", &timetable)?;

                    println!("Starting bench of live algorithms..");
                    benchmarking::bench_algorithms_live("IFF", &timetable, &updates)?;
                }
                Some("trainline") => {
                    println!("Generating timetable and updates list, this might take a while...");
                    let timetable = data::generic_data::get_data()?;

                    println!("The timetable contains {} connections, stopping at {} places.", 
                        &timetable.trips.iter().map(|t| t.connections.len()).sum::<usize>(),
                        &timetable.stops.len()
                    );

                    println!("Starting bench of static algorithms..");
                    benchmarking::bench_algorithms("Trainline EU", &timetable)?;
                }
                _ => {}
            }

        }
        ("trip", Some(sub_matches)) => {
            let id = sub_matches.value_of("id").unwrap().parse().unwrap();
            println!("Looking up route of service {}", id);

            let timetable = iff::get_timetable_for_day(&NaiveDate::from_ymd(2021, 1, 15))?;
            let trip = timetable.trips.iter().find(|x| x.identifier == id).unwrap();

            for conn in &trip.connections {
                println!("{:?} at {:?} => {:?} at {:?}", timetable.stops.get(&conn.dep_stop).unwrap(), conn.dep_time, timetable.stops.get(&conn.arr_stop).unwrap(), conn.arr_time);
            }
        }
        ("example", _) => {
            // Performs an example routing with CSA Vec
            let date = NaiveDate::from_ymd(2021, 1, 15);

            println!("Loading timetable for {:?}", date);
            let timetable = iff::get_timetable_for_day(&date)?;

            // Find ID's of amf and esk
            let amf = timetable.stops.iter().find(|(_, stop)| stop.to_string() == "amf").unwrap().0;
            let esk = timetable.stops.iter().find(|(_, stop)| stop.to_string() == "esk").unwrap().0;

            let mut alg = algorithms::td_simple_btree::TDSimpleBTree::new(&timetable);
            let route = alg.find_earliest_arrival(*esk, *amf, Local.ymd(2021, 1, 15).and_hms(13, 0, 0).timestamp() as u32).unwrap();
            println!("{}", route.format_fancy(&timetable.stops));

            // Get changes
            let updates = info_plus::read_dvs_to_updates(&date)?;
            for update in updates.iter() {
                alg.update(update);
            }

            println!("After updating:");
            let route = alg.find_earliest_arrival(*esk, *amf, Local.ymd(2021, 1, 15).and_hms(13, 0, 0).timestamp() as u32).unwrap();
            println!("{}", route.format_fancy(&timetable.stops));

        }
        _ => {},
    }

    Ok(())
}