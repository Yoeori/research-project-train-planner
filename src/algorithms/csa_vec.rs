use crate::{benchable::Benchable, types::{Connection, Timetable, TripResult}};

pub const MAX_STATIONS: usize = 100000;

#[derive(Debug)]
pub struct CSAVec<'a> {
    timetable: Vec<&'a Connection>,
}

fn main_loop(timetable: &[&Connection], arr_stop: usize, earliest_arrival: &mut [u32], in_connection: &mut [usize]) {
    let mut earliest = std::u32::MAX;

    for (i, connection) in timetable.iter().enumerate() {
        if connection.dep_time >= earliest_arrival[connection.dep_stop] &&
                connection.arr_time < earliest_arrival[connection.arr_stop] {
            earliest_arrival[connection.arr_stop] = connection.arr_time;
            in_connection[connection.arr_stop] = i;

            if connection.arr_stop == arr_stop && connection.arr_time < earliest {
                earliest = connection.arr_time;
            }
        } else if connection.arr_time > earliest {
            break;
        }
    }
}

fn get_result<'a>(timetable: &'a Vec<&Connection>, in_connection: &[usize], arrival_station: usize) -> Option<TripResult<'a>> {
    if in_connection[arrival_station] == std::u32::MAX as usize {
        None
    } else {
        let mut route = Vec::new();
        let mut last_connection_index = in_connection[arrival_station];

        while last_connection_index != std::u32::MAX as usize {
            let ref connection = timetable[last_connection_index];
            route.push(*connection);
            last_connection_index = in_connection[connection.dep_stop];
        }

        route.reverse();

        return Some(TripResult {
            connections: route
        });
    }
}

// Based on https://github.com/trainline-eu/csa-challenge/blob/master/csa.rs (WTFPL license)
impl<'a> Benchable<'a> for CSAVec<'a> {
    fn name(&self) -> &'static str {
        "Connection Scan Algorithm with Vec"
    }
    
    fn new(trips: &'a Timetable) -> Self {
        let mut timetable = vec![];
        for trip in &trips.trips {
            timetable.extend(&trip.connections);
        }

        timetable.sort();

        CSAVec {
            timetable
        }
    }

    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult> {

        let mut in_connection = vec!(std::u32::MAX as usize; MAX_STATIONS);
        let mut earliest_arrival = vec!(std::u32::MAX; MAX_STATIONS);

        earliest_arrival[dep_stop as usize] = dep_time as u32;

        if dep_stop < MAX_STATIONS && arr_stop < MAX_STATIONS {
            main_loop(&self.timetable[..], arr_stop, &mut earliest_arrival, &mut in_connection);
        }

        get_result(&self.timetable, &in_connection, arr_stop)
    }
}

alg_test!(CSAVec);