use std::{collections::{HashMap, HashSet}, ops::Range};

use chrono::NaiveDate;
use diesel::Insertable;
use smol_str::SmolStr;

use crate::database::schema::stations;

#[derive(Debug, Clone)]
pub struct IFF {
    /// Map of all stations with Station code as key (TODO UIC code as key)
    pub stations: HashMap<SmolStr, Station>,
    
    pub trns_modes: HashMap<SmolStr, String>,
    pub services: HashMap<usize, Service>,
    pub validity: HashMap<usize, HashSet<NaiveDate>>
}

#[derive(Debug, Clone, Queryable, Insertable)]
#[table_name = "stations"]
pub struct Station {
    #[diesel(deserialize_as = "String")]
    pub code: String,
    
    #[diesel(deserialize_as = "String")]
    pub name: String,

    #[diesel(deserialize_as = "String")]
    pub country: String,
    pub timezone: i8,

    pub interchange_duration: u8,
    pub is_interchange_station: bool,

    pub lat: i32,
    pub lng: i32,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub identification: usize,
    pub service_number: Vec<(usize, Range<usize>)>,
    
    pub validity: usize,

    pub trns_modes: Vec<String>,
    pub attributes: Vec<String>,

    pub stops: Vec<Stop>,
}

#[derive(Debug, Clone)]
pub enum Stop {
    Departure { dep_time: u16, station: String, track: Option<String> },
    ShortIntermediate { dep_time: u16, station: String, track: Option<String> },
    Intermediate { arr_time: u16, dep_time: u16, station: String, track: Option<String> },
    Pass { station: String },
    Arrival { arr_time: u16, station: String, track: Option<String> }
}