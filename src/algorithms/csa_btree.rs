use std::collections::{BTreeSet, HashMap};

use crate::{benchable::{Benchable, BenchableLive}, types::{Connection, Timetable, TripPart, TripResult, TripUpdate}};

pub const MAX_STATIONS: usize = 100000;

#[derive(Debug)]
pub struct CSABTree<'a> {
    connections: BTreeSet<&'a Connection>,
    footpaths: &'a HashMap<usize, Vec<(usize, u32)>>
}

// Based on https://github.com/trainline-eu/csa-challenge/blob/master/csa.rs (WTFPL license)
impl<'a> Benchable<'a> for CSABTree<'a> {
    fn name(&self) -> &'static str {
        "Connection Scan Algorithm with BTree"
    }

    fn new(timetable: &'a Timetable) -> Self {
        let mut connections = BTreeSet::new();
        for trip in &timetable.trips {
            connections.extend(&trip.connections);
        }

        CSABTree {
            connections,
            footpaths: &timetable.footpaths
        }
    }

    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult> {
        let mut earliest_arrival = vec!(std::u32::MAX; MAX_STATIONS);
        let mut in_connection = vec!(None; MAX_STATIONS);
        let mut journeys = vec!(None; MAX_STATIONS);

        for &(f_stop, dur) in self.footpaths.get(&dep_stop).unwrap() {
            earliest_arrival[f_stop] = dep_time + dur;
        }

        for &conn in self.connections.range(Connection {
            dep_stop: MAX_STATIONS,
            arr_stop: MAX_STATIONS,
            dep_time,
            arr_time: dep_time,
            trip_id: 0
        }..) {
            if earliest_arrival[arr_stop] <= conn.dep_time {
                break;
            }

            if in_connection[conn.trip_id].is_some() || earliest_arrival[conn.dep_stop] <= conn.dep_time {
                if in_connection[conn.trip_id].is_none() {
                    in_connection[conn.trip_id] = Some(conn);
                }

                for &(f_stop, dur) in self.footpaths.get(&conn.arr_stop).unwrap() {
                    if conn.arr_time + dur < earliest_arrival[f_stop] {
                        earliest_arrival[f_stop] = conn.arr_time + dur;
                        journeys[f_stop] = Some((in_connection[conn.trip_id].unwrap(), conn, (conn.arr_stop, f_stop, dur)))
                    }
                }
            }
        }

        let mut journey = vec![];
        let mut cur = arr_stop;
        while let Some((con1, con2, footpath)) = journeys[cur] {
            journey.push(TripPart::Footpath(footpath.0, footpath.1, footpath.2));
            journey.push(TripPart::Connection(con1, con2));
            cur = con1.dep_stop;
        }

        journey.reverse();

        // We do not care about the final footpath
        journey.remove(journey.len()-1);

        return Some(TripResult {
            parts: journey
        });
    }

}

impl<'a> BenchableLive<'a> for CSABTree<'a> {
    fn update(&mut self, update: &'a TripUpdate) {
        match update {
            TripUpdate::DeleteTrip { trip } => {
                for conn in trip.connections.iter() {
                    self.connections.remove(conn);
                }
            }
            TripUpdate::AddTrip { trip } => {
                for conn in trip.connections.iter() {
                    self.connections.insert(conn);
                }
            }
            TripUpdate::AddConnection { old_trip: _, new_trip: _, connection } => {
                self.connections.insert(connection);
            }
            TripUpdate::DeleteConnection { old_trip: _, new_trip: _, connection } => {
                self.connections.remove(connection);
            }
        }
    }
}

alg_test!(CSABTree);