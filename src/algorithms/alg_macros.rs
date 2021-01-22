#[macro_export]
macro_rules! alg_test {
    ($x:ident) => {
        #[cfg(test)]
        mod alg_tests {
            use super::*;

            #[test]
            fn route_test() {
                use crate::types::{Timetable, Connection, TripResult, Trip};
                use std::collections::HashMap;
                // Very simple test, this doesnt prove anything :(
                let connections = vec![
                    Connection { dep_stop: 0, arr_stop: 1, dep_time: 1, arr_time: 4, trip_id: 0 },
                    Connection { dep_stop: 1, arr_stop: 2, dep_time: 5, arr_time: 9, trip_id: 0 },
                    Connection { dep_stop: 2, arr_stop: 3, dep_time: 10, arr_time: 14, trip_id: 0 },
                    Connection { dep_stop: 3, arr_stop: 4, dep_time: 15, arr_time: 19, trip_id: 0 },
                    Connection { dep_stop: 4, arr_stop: 5, dep_time: 20, arr_time: 25, trip_id: 0 },
                ];

                let timetable = Timetable {
                    stops: HashMap::new(),
                    trips: vec![Trip {
                        identifier: 0,
                        connections
                    }]
                };

                let alg = $x::new(&timetable);

                let result = alg.find_earliest_arrival(0, 5, 0);

                assert!(result.is_some());
                assert_eq!(alg.find_earliest_arrival(0, 5, 0).unwrap(), TripResult {
                    connections: timetable.trips[0].connections.iter().collect::<Vec<&Connection>>()
                });
            }
        }
    }
}

// TODO: Testing for live and profile algorithms.