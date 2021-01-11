#![allow(unused_imports)]
pub mod schema;
pub mod types;

use std::{env, error::Error};

use diesel::prelude::*;
use diesel::mysql::MysqlConnection;
use dotenv::dotenv;

pub fn establish_connection() -> MysqlConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");
    MysqlConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}