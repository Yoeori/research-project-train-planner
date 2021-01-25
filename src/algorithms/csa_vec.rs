use std::collections::HashMap;

use crate::{benchable::Benchable, types::{Connection, Timetable, TripPart, TripResult}};

pub const MAX_STATIONS: usize = 100000;

#[derive(Debug)]
pub struct CSAVec<'a> {
    connections: Vec<&'a Connection>,
    footpaths: &'a HashMap<usize, Vec<(usize, u32)>>
}

// Based on https://github.com/trainline-eu/csa-challenge/blob/master/csa.rs (WTFPL license)
impl<'a> Benchable<'a> for CSAVec<'a> {
    fn name(&self) -> &'static str {
        "CSA with Vec"
    }

    fn new(timetable: &'a Timetable) -> Self {
        let mut connections = vec![];
        for trip in &timetable.trips {
            connections.extend(&trip.connections);
        }

        connections.sort();

        CSAVec {
            connections,
            footpaths: &timetable.footpaths
        }
    }

    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult> {
        let mut earliest_arrival = vec!(std::u32::MAX; MAX_STATIONS);
        let mut in_connection = vec!(None; MAX_STATIONS * 10);
        let mut journeys = HashMap::new();

        for &(f_stop, dur) in self.footpaths.get(&dep_stop).unwrap() {
            earliest_arrival[f_stop] = dep_time + dur;
        }

        // TODO binary tree for start
        for &conn in &self.connections {
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
                        journeys.insert(f_stop, (in_connection[conn.trip_id].unwrap(), conn, (conn.arr_stop, f_stop, dur)));
                    }
                }
            }
        }

        let mut journey = vec![];
        let mut cur = arr_stop;
        while let Some((con1, con2, footpath)) = journeys.get(&cur) {
            journey.push(TripPart::Footpath(footpath.0, footpath.1, footpath.2));
            journey.push(TripPart::Connection(con1, con2));
            cur = con1.dep_stop;
        }

        if journey.is_empty() {
            return None;
        }

        journey.reverse();

        // We do not care about the final footpath
        journey.remove(journey.len()-1);

        return Some(TripResult {
            parts: journey
        });
    }

}

alg_test!(CSAVec);