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
        _ => {},
    }

    Ok(())
}