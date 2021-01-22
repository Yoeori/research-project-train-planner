// This file will define all struct and methods for a timetable
// Timetable that is described here can be found in the proposal.
// A timetable consists of a list of connections, trips, stops and footpaths
// A connection consists of a departure stop, arrival stop, departure time and arrival time;
// A trip is a ordered list of connection which follow each other from stop to stop and contains an ID unique for the timetable date 
//  (index for connection, id for trip)
// A stop can be anything where connections arrive, this information can be very detailed (specific platfdorm 2a) to simple like having
//  only one stop for a set of 
// A timetable can also be updated with live information, either changing connections, deleting them or adding new connections (and associated trips)

use std::{cmp::Ordering, collections::HashMap, error::Error, fmt::Debug};
use std::hash::Hash;

/// As defined
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Connection {
    pub dep_stop: usize,
    pub arr_stop: usize,
    
    pub dep_time: u32,
    pub arr_time: u32,

    pub trip_id: usize
}

impl Ord for Connection {
    fn cmp(&self, other: &Connection) -> Ordering {
        self.dep_time.cmp(&other.dep_time)
            .then_with(|| self.arr_time.cmp(&other.arr_time))
            .then_with(|| self.dep_stop.cmp(&other.dep_stop))
            .then_with(|| self.arr_stop.cmp(&other.arr_stop))
    }
}

impl PartialOrd for Connection {
    fn partial_cmp(&self, other: &Connection) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Connection {
    pub fn parse_from_string(line: &str, trip_id: usize) -> Result<Connection, Box<dyn Error>> {
        let mut splitted = line.split(" ").map(|t| { t.parse::<u32>() });

        Ok(Connection {
            dep_stop: splitted.next().ok_or("Missing dep_stop")?? as usize,
            arr_stop: splitted.next().ok_or("Missing arr_stop")?? as usize,
            dep_time: splitted.next().ok_or("Missing dep_time")??,
            arr_time: splitted.next().ok_or("Missing arr_time")??,
            trip_id
        })
    }
}

// As defined (added identifier for 'easier' identification)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Trip {
    pub identifier: usize,
    pub connections: Vec<Connection>
}

// As defined, however trip based for identification purposes
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TripUpdate {
    DeleteTrip { trip: Trip },
    AddTrip { trip: Trip },
    AddConnection { trip: Trip, connection: Connection },
    DeleteConnection { trip: Trip, connection: Connection },
    UpdateConnection { trip: Trip, connection_old: Connection, connection_new: Connection },
}

// As defined
#[derive(Debug)]
pub struct Path {
    from: u32,
    to: u32,
    duration: u8
}

pub trait Stop: Debug {
    // Since in the timetable all stops are converted to dyn Stop, it's hard to find stops by name
    // We probably need to figure out a way such that all stop ID's are derived from the stop itself
    // Most likely solution would be to use UIC station code + some kind of platform identifier
    // Might also want to look to change the timetable to use a generic stop type instead of a dynamic
    // However, multiple types of stop might also be possible, think of the Norwegian model containing MultimodalStopPlace and GroupOfStopPlaces etc.
    fn to_string(&self) -> String;
    fn coords(&self) -> Option<(f64, f64)>;
    fn distance(&self, other: &Box<dyn Stop>) -> Option<f64>;
}

#[derive(Debug)]
pub struct Timetable {
    pub stops: HashMap<usize, Box<dyn Stop>>,
    pub trips: Vec<Trip>,
    //pub footpaths: HashMap<usize, HashMap<usize, usize>>
    // Connections and stations are defined within the trips and footpaths.
}

// TODO this will later be an tick-tock pair of (Trip, connection index start, connection index end) and footpaths
// For no there are not footpaths
#[derive(Debug, PartialEq, Eq)]
pub struct TripResult<'a> {
    pub connections: Vec<&'a Connection>
}

impl TripResult<'_> {
    #[allow(dead_code)]
    pub fn arrival(&self) -> u32 {
        self.connections.last().unwrap().arr_time
    }
}