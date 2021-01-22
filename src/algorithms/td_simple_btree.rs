use std::collections::{BTreeSet, HashMap};
use std::collections::BinaryHeap;
use std::cmp::Ordering;

use crate::{benchable::{Benchable, BenchableLive}, types::{Timetable, TripResult, TripUpdate}};
use crate::types::Connection;

pub const MAX_STATIONS: usize = 100000;

#[derive(Debug)]
pub struct Station<'a> {
    station: usize,
    neighbours: HashMap<usize, BTreeSet<&'a Connection>>
}

impl<'a> Station<'a> {
    fn add_connection(&mut self, conn: &'a Connection) {
        if !self.neighbours.contains_key(&conn.arr_stop) {
            self.neighbours.insert(conn.arr_stop, BTreeSet::new());
        }

        self.neighbours.get_mut(&conn.arr_stop).unwrap().insert(conn);
    }
}

/// Finds the first connection in vec which is equal or greater than start_time
fn bin_search_arr<'a>(connections: &BTreeSet<&'a Connection>, start_time: u32) -> Option<&'a Connection> {
    connections.range(Connection {
        dep_time: start_time,
        arr_time: 0,
        dep_stop: 0,
        arr_stop: 0,
        trip_id: 0
    }..).next().map(|x| *x)
}

#[derive(Debug)]
pub struct TDSimpleBTree<'a> {
    data: HashMap<usize, Station<'a>>
}

impl<'a> Benchable<'a> for TDSimpleBTree<'a> {
    fn name(&self) -> &'static str {
        "Simple time-dependent graph approach with Binary Tree"
    }

    fn new(trips: &'a Timetable) -> Self {
        let mut stations: HashMap<usize, Station> = HashMap::new();

        for connection in trips.trips.iter().map(|t| &t.connections).flatten() {
            if !stations.contains_key(&connection.dep_stop) {
                stations.insert(connection.dep_stop, Station {
                    station: connection.dep_stop,
                    neighbours: HashMap::new()
                });
            }

            stations.get_mut(&connection.dep_stop).unwrap().add_connection(connection);        
        }

        TDSimpleBTree {
            data: stations
        }
    }

    // Dijkstra implementation is mainly derived from example at: https://doc.rust-lang.org/std/collections/binary_heap/
    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult> {

        #[derive(Copy, Clone, Eq, PartialEq)]
        struct State {
            cost: u32,
            station: usize,
        }

        // The priority queue depends on `Ord`.
        // Explicitly implement the trait so the queue becomes a min-heap
        // instead of a max-heap.
        impl Ord for State {
            fn cmp(&self, other: &State) -> Ordering {
                // Notice that the we flip the ordering on costs.
                // In case of a tie we compare positions - this step is necessary
                // to make implementations of `PartialEq` and `Ord` consistent.
                other.cost.cmp(&self.cost)
                    .then_with(|| self.station.cmp(&other.station))
            }
        }

        // `PartialOrd` needs to be implemented as well.
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &State) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut dist: Vec<u32> = vec![u32::MAX; MAX_STATIONS];
        let mut heap: BinaryHeap<State> = BinaryHeap::new();
        let mut prev: Vec<Option<&Connection>> = vec![None; MAX_STATIONS];

        dist[dep_stop] = dep_time;
        heap.push(State {
            cost: dep_time,
            station: dep_stop
        });

        while let Some(State { cost, station }) = heap.pop() {

            // Alternatively we could have continued to find shortest paths for whole station graph
            if station == arr_stop {
                // Create trip
                let mut trip: Vec<&Connection> = Vec::new();
                let mut cur = arr_stop;
                while let Some(conn) = prev[cur] {
                    trip.push(conn);
                    cur = conn.dep_stop;
                }

                trip.reverse();

                return Some(TripResult {
                    connections: trip
                });
            }

            // Important as we may have already found a better way
            if cost > dist[station] || !self.data.contains_key(&station) { continue; }

            // For each node we can reach, see if we can find a way with
            // a lower cost going through this node
            for (_, node) in self.data.get(&station).unwrap().neighbours.iter() {

                // Find cheapest path to edge station
                let edge = bin_search_arr(node, cost);

                if let Some(edge) = edge {
                    if edge.arr_time < dist[edge.arr_stop] {
                        heap.push(State { cost: edge.arr_time, station: edge.arr_stop });
                        dist[edge.arr_stop] = edge.arr_time;
                        prev[edge.arr_stop] = Some(edge);
                    }
                }
            }
        };

        None
    }
}

impl<'a> BenchableLive<'a> for TDSimpleBTree<'a> {
    fn update(&mut self, update: &'a TripUpdate) {
        fn delete_connection<'a>(benchable: &mut TDSimpleBTree<'a>, conn: &'a Connection) {
            if let Some(station) = benchable.data.get_mut(&conn.dep_stop) {
                if let Some(connections) = station.neighbours.get_mut(&conn.arr_stop) {
                    connections.remove(&conn);
                }
            }
        }

        fn add_connection<'a>(benchable: &mut TDSimpleBTree<'a>, conn: &'a Connection) {
            if let Some(station) = benchable.data.get_mut(&conn.dep_stop) {
                station.add_connection(conn);
            }
        }

        match update {
            TripUpdate::DeleteTrip { trip } => {
                for conn in trip.connections.iter() {
                    delete_connection(self, conn);
                }
            }
            TripUpdate::AddTrip { trip } => {
                for conn in trip.connections.iter() {
                    add_connection(self, conn);
                }
            }
            TripUpdate::AddConnection { trip: _, connection } => {
                add_connection(self, connection);
            }
            TripUpdate::DeleteConnection { trip: _, connection } => {
                delete_connection(self, connection);
            }
            TripUpdate::UpdateConnection { trip: _, connection_old, connection_new } => {
                delete_connection(self, connection_old);
                add_connection(self, connection_new);
            }
        }
    }
}

alg_test!(TDSimpleBTree);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bin_search() {      
        let mut connections1 = BTreeSet::new();
        connections1.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 0, arr_time: 0, trip_id: 0 });
        connections1.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 5, arr_time: 0, trip_id: 0 });
        connections1.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 10, arr_time: 0, trip_id: 0 });
        connections1.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 15, arr_time: 0, trip_id: 0 });
        connections1.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 20, arr_time: 0, trip_id: 0 });

        let mut connections2 = BTreeSet::new();
        connections2.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 0, arr_time: 0, trip_id: 0 });
        connections2.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 5, arr_time: 0 , trip_id: 0});
        connections2.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 10, arr_time: 0, trip_id: 0 });
        connections2.insert(&Connection { dep_stop: 0, arr_stop: 0, dep_time: 15, arr_time: 0, trip_id: 0 });

        assert_eq!(bin_search_arr(&connections1, 0).unwrap().dep_time, 0);
        assert_eq!(bin_search_arr(&connections1, 20).unwrap().dep_time, 20);
        assert_eq!(bin_search_arr(&connections1, 19).unwrap().dep_time, 20);
        assert_eq!(bin_search_arr(&connections1, 6).unwrap().dep_time, 10);
        assert_eq!(bin_search_arr(&connections1, 10).unwrap().dep_time, 10);

        assert_eq!(bin_search_arr(&connections2, 0).unwrap().dep_time, 0);
        assert_eq!(bin_search_arr(&connections2, 15).unwrap().dep_time, 15);
        assert_eq!(bin_search_arr(&connections2, 14).unwrap().dep_time, 15);
        assert_eq!(bin_search_arr(&connections2, 4).unwrap().dep_time, 5);
        assert_eq!(bin_search_arr(&connections2, 5).unwrap().dep_time, 5);

        assert!(bin_search_arr(&connections1, 21).is_none());

    }
}