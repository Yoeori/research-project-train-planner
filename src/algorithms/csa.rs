use crate::types::{Connection, TripResult};

pub const MAX_STATIONS: usize = 100000;

// Based on https://github.com/trainline-eu/csa-challenge/blob/master/csa.rs
fn main_loop(timetable: &[Connection], arrival_station: usize, earliest_arrival: &mut [u32], in_connection: &mut [usize]) {
    let mut earliest = std::u32::MAX;

    for (i, connection) in timetable.iter().enumerate() {
        if connection.dep_time >= earliest_arrival[connection.dep_stop] &&
                connection.arr_time < earliest_arrival[connection.arr_stop] {
            earliest_arrival[connection.arr_stop] = connection.arr_time;
            in_connection[connection.arr_stop] = i;

            if connection.arr_stop == arrival_station && connection.arr_time < earliest {
                earliest = connection.arr_time;
            }
        } else if connection.arr_time > earliest {
            break;
        }
    }
}

fn get_result<'a>(timetable: &'a Vec<Connection>, in_connection: &[usize], arrival_station: usize) -> Option<TripResult<'a>> {
    if in_connection[arrival_station] == std::u32::MAX as usize {
        None
    } else {
        let mut route = Vec::new();
        let mut last_connection_index = in_connection[arrival_station];

        while last_connection_index != std::u32::MAX as usize {
            let ref connection = timetable[last_connection_index];
            route.push(connection);
            last_connection_index = in_connection[connection.dep_stop];
        }

        route.reverse();

        return Some(TripResult {
            connections: route
        });
    }
}

pub fn compute<'a>(timetable: &'a Vec<Connection>, departure_station: usize, arrival_station: usize, departure_time: u32) -> Option<TripResult<'a>> {
    let mut in_connection = vec!(std::u32::MAX as usize; MAX_STATIONS);
    let mut earliest_arrival = vec!(std::u32::MAX; MAX_STATIONS);

    earliest_arrival[departure_station as usize] = departure_time;

    if departure_station < MAX_STATIONS && arrival_station < MAX_STATIONS {
        main_loop(&timetable, arrival_station, &mut earliest_arrival, &mut in_connection);
    }

    get_result(&timetable, &in_connection, arrival_station)
}