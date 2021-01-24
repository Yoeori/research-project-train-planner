// This file will define all struct and methods for a timetable
// Timetable that is described here can be found in the proposal.
// A timetable consists of a list of connections, trips, stops and footpaths
// A connection consists of a departure stop, arrival stop, departure time and arrival time;
// A trip is a ordered list of connection which follow each other from stop to stop and contains an ID unique for the timetable date 
//  (index for connection, id for trip)
// A stop can be anything where connections arrive, this information can be very detailed (specific platfdorm 2a) to simple like having
//  only one stop for a set of 
// A timetable can also be updated with live information, either changing connections, deleting them or adding new connections (and associated trips)

use std::{cmp::Ordering, collections::HashMap, error::Error, fmt::{self, Debug}};
use std::hash::Hash;

use chrono::{Local, TimeZone};

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

impl Ord for Trip {
    fn cmp(&self, other: &Trip) -> Ordering {
        self.connections[0].dep_time.cmp(&other.connections[0].dep_time)
            .then_with(|| self.identifier.cmp(&other.identifier))
    }
}

impl PartialOrd for Trip {
    fn partial_cmp(&self, other: &Trip) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// As defined, however trip based for identification purposes
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TripUpdate {
    DeleteTrip { trip: Trip },
    AddTrip { trip: Trip },
    AddConnection { old_trip: Trip, new_trip: Trip, connection: Connection },
    DeleteConnection { old_trip: Trip, new_trip: Trip, connection: Connection },
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
    pub footpaths: HashMap<usize, Vec<(usize, u32)>> // Stop a to stop b => time
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TripPart<'a> {
    Connection(&'a Connection, &'a Connection),
    Footpath(usize, usize, u32)
}

impl TripPart<'_> {
    fn from(&self) -> usize {
        match self {
            TripPart::Connection(a, _) => a.dep_stop,
            TripPart::Footpath(a, _, _) => *a
        }
    }

    fn to(&self) -> usize {
        match self {
            TripPart::Connection(_, b) => b.arr_stop,
            TripPart::Footpath(_, b, _) => *b
        }
    }

    fn format_fancy(&self, stops: &HashMap<usize, Box<dyn Stop>>) -> String {
        match self {
            TripPart::Connection(a, b) => format!(
                "Depart from {} at {}, and arrive at {} at {}", 
                stops.get(&a.dep_stop).unwrap().to_string(), Local.timestamp(a.dep_time as i64, 0),
                stops.get(&b.arr_stop).unwrap().to_string(), Local.timestamp(b.arr_time as i64, 0)
            ),
            TripPart::Footpath(a, b, duration) => format!(
                "Walk from {} to {} taking {} mins",
                stops.get(a).unwrap().to_string(), stops.get(b).unwrap().to_string(), duration / 60
            )
        }
    }
}

impl fmt::Display for TripPart<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TripPart::Connection(a, b) => write!(
                f, "Depart from {} at {}, and arrive at {} at {}", 
                a.dep_stop, a.dep_time, b.arr_stop, b.arr_time
            ),
            TripPart::Footpath(a, b, duration) => write!(
                f, "Walk from {} to {} taking {} mins",
                a, b, duration / 60
            )
        }
    }
}

// Journey as defined in the paper
#[derive(Debug, PartialEq, Eq)]
pub struct TripResult<'a> {
    pub parts: Vec<TripPart<'a>>
}

impl TripResult<'_> {
    #[allow(dead_code)]
    pub fn arrival(&self) -> u32 {
        if let Some(TripPart::Connection(_, b)) = self.parts.last() {
            return b.arr_time;
        }

        panic!("Trip result did not contain a final connection!");
    }

    pub fn format_fancy(&self, stops: &HashMap<usize, Box<dyn Stop>>) -> String {
        let mut res = format!(
            "Trip from {} to {}\n", 
            stops.get(&self.parts.first().unwrap().from()).unwrap().to_string(), 
            stops.get(&self.parts.last().unwrap().to()).unwrap().to_string()
        );

        for part in &self.parts {
            res.push_str(&format!("{}\n", part.format_fancy(stops))[..]);
        }

        res
    }
}

impl fmt::Display for TripResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Trip from {} to {}\n", self.parts.first().unwrap().from(), self.parts.last().unwrap().to())?;
        for part in &self.parts[..(self.parts.len()-1)] {
            write!(f, "{}\n", part)?
        }

        write!(f, "{}", self.parts[self.parts.len()-1])
    }
}