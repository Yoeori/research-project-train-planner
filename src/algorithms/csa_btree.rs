use std::collections::BTreeSet;

use crate::{benchable::{Benchable, BenchableLive}, types::{Connection, Timetable, TripResult, TripUpdate}};

pub const MAX_STATIONS: usize = 100000;

fn main_loop<'a>(timetable: impl Iterator<Item = &'a &'a Connection>, arr_stop: usize, earliest_arrival: &mut [u32], in_connection: &mut [Option<&'a Connection>]) {
    let mut earliest = std::u32::MAX;

    for connection in timetable {
        if connection.dep_time >= earliest_arrival[connection.dep_stop] &&
                connection.arr_time < earliest_arrival[connection.arr_stop] {
            earliest_arrival[connection.arr_stop] = connection.arr_time;
            in_connection[connection.arr_stop] = Some(&connection);

            if connection.arr_stop == arr_stop && connection.arr_time < earliest {
                earliest = connection.arr_time;
            }
        } else if connection.arr_time > earliest {
            break;
        }
    }
}

fn get_result<'a>(in_connection: &[Option<&'a Connection>], arrival_station: usize) -> Option<TripResult<'a>> {
    if in_connection[arrival_station] == None {
        None
    } else {
        let mut route = Vec::new();

        let mut last_station = arrival_station;
        while let Some(connection) = in_connection[last_station] {
            route.push(connection);
            last_station = connection.dep_stop;
        }

        route.reverse();

        return Some(TripResult {
            connections: route
        });
    }
}

#[derive(Debug)]
pub struct CSABTree<'a> {
    timetable: BTreeSet<&'a Connection>
}

// Based on https://github.com/trainline-eu/csa-challenge/blob/master/csa.rs (WTFPL license)
impl<'a> Benchable<'a> for CSABTree<'a> {
    fn name(&self) -> &'static str {
        "Connection Scan Algorithm with BTree"
    }

    fn new(timetable: &'a Timetable) -> Self {
        let mut our_timetable = BTreeSet::new();
        for trip in &timetable.trips {
            our_timetable.extend(&trip.connections);
        }

        CSABTree {
            timetable: our_timetable
        }
    }

    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult> {

        let mut in_connection = vec!(None; MAX_STATIONS);
        let mut earliest_arrival = vec!(std::u32::MAX; MAX_STATIONS);

        earliest_arrival[dep_stop as usize] = dep_time as u32;

        if dep_stop < MAX_STATIONS && arr_stop < MAX_STATIONS {
            main_loop(self.timetable.range(Connection {
                dep_stop: MAX_STATIONS,
                arr_stop: MAX_STATIONS,
                dep_time: dep_time,
                arr_time: dep_time
            }..), arr_stop, &mut earliest_arrival, &mut in_connection);
        }

        get_result(&in_connection, arr_stop)
    }

}

impl<'a> BenchableLive<'a> for CSABTree<'a> {
    fn update(&mut self, update: &'a TripUpdate) {
        match update {
            TripUpdate::DeleteTrip { trip } => {
                for conn in trip.connections.iter() {
                    self.timetable.remove(conn);
                }
            }
            TripUpdate::AddTrip { trip } => {
                for conn in trip.connections.iter() {
                    self.timetable.insert(conn);
                }
            }
            TripUpdate::AddConnection { trip: _, connection } => {
                self.timetable.insert(connection);
            }
            TripUpdate::DeleteConnection { trip: _, connection } => {
                self.timetable.remove(connection);
            }
            TripUpdate::UpdateConnection { trip: _, connection_old, connection_new } => {
                self.timetable.remove(connection_old);
                self.timetable.insert(connection_new);
            }
        }
    }
}

alg_test!(CSABTree);