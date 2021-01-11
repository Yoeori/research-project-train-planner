// This file will define all struct and methods for a timetable
// Timetable that is described here can be found in the proposal.
// A timetable consists of a list of connections, trips, stops and footpaths
// A connection consists of a departure stop, arrival stop, departure time and arrival time;
// A trip is a ordered list of connection which follow each other from stop to stop and contains an ID unique for the timetable date 
//  (index for connection, id for trip)
// A stop can be anything where connections arrive, this information can be very detailed (specific platfdorm 2a) to simple like having
//  only one stop for a set of 
// A timetable can also be updated with live information, either changing connections, deleting them or adding new connections (and associated trips)

use std::{cmp::Ordering, error::Error};

/// As defined
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Connection {
    pub dep_stop: usize,
    pub arr_stop: usize,
    
    pub dep_time: u32,
    pub arr_time: u32
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
    pub fn parse_from_string(line: &str) -> Result<Connection, Box<dyn Error + 'static>> {
        let mut splitted = line.split(" ").map(|t| { t.parse::<u32>() });

        Ok(Connection {
            dep_stop: splitted.next().ok_or("Missing dep_stop")?? as usize,
            arr_stop: splitted.next().ok_or("Missing arr_stop")?? as usize,
            dep_time: splitted.next().ok_or("Missing dep_time")??,
            arr_time: splitted.next().ok_or("Missing arr_time")??,
        })
    }
}

// As defined (added identifier for 'easier' identification)
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Trip {
    identifier: usize,
    connections: Vec<Connection>
}

// As defined, however trip based for identification purposes
#[derive(Debug)]
#[allow(dead_code)]
pub enum TripUpdate<'a> {
    DeleteTrip { identifier: usize },
    DeleteConnection { identifier: usize, index: usize, stop: u32 },
    UpdateConnection { identifier: usize, index: usize, connection: &'a Connection },
    AddConnection { identifier: usize, index: usize, connection: &'a Connection }
}

// As defined
#[derive(Debug)]
pub struct Path {
    from: u32,
    to: u32,
    duration: u8
}

// TODO this will later be an tick-tock pair of (Trip, connection index start, connection index end) and walk path
#[derive(Debug, PartialEq, Eq)]
pub struct TripResult<'a> {
    pub connections: Vec<&'a Connection>
}

impl TripResult<'_> {
    pub fn arrival(&self) -> u32 {
        self.connections.last().unwrap().arr_time
    }
}