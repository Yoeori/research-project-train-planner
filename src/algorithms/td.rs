use std::collections::HashMap;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

use crate::types::TripResult;
use crate::types::Connection;

pub const MAX_STATIONS: usize = 100000;

#[derive(Debug)]
pub struct Station<'a> {
    station: usize,
    neighbours: HashMap<usize, Vec<&'a Connection>>
}

impl<'a> Station<'a> {

    fn add_connection(&mut self, conn: &'a Connection) {
        if !self.neighbours.contains_key(&conn.arr_stop) {
            self.neighbours.insert(conn.arr_stop, Vec::new());
        }

        self.neighbours.get_mut(&conn.arr_stop).unwrap().push(conn);
    }

    fn sort(&mut self) {
        for (_, station) in self.neighbours.iter_mut() {
            station.sort();
        }
    }

}

pub fn prepare<'a>(connections: &'a Vec<Connection>) -> HashMap<usize, Station> {
    let mut stations: HashMap<usize, Station> = HashMap::new();

    for connection in connections {
        if !stations.contains_key(&connection.dep_stop) {
            stations.insert(connection.dep_stop, Station {
                station: connection.dep_stop,
                neighbours: HashMap::new()
            });
        }

        stations.get_mut(&connection.dep_stop).unwrap().add_connection(connection);        
    }

    for (_, station) in stations.iter_mut() {
        station.sort();
    }

    stations
}

// Dijkstra implementation is mainly derived from example at: https://doc.rust-lang.org/std/collections/binary_heap/
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


pub fn compute<'a>(data: &'a HashMap<usize, Station>, start_stop: usize, end_stop: usize, start_time: u32) -> Option<TripResult<'a>> {
 
    let mut dist: Vec<u32> = vec![u32::MAX; MAX_STATIONS];
    let mut heap: BinaryHeap<State> = BinaryHeap::new();
    let mut prev: Vec<Option<&Connection>> = vec![None; MAX_STATIONS];

    dist[start_stop] = 0;
    heap.push(State {
        cost: start_time,
        station: start_stop
    });

    while let Some(State { cost, station }) = heap.pop() {
        // Alternatively we could have continued to find all shortest paths
        if station == end_stop {
            // Create trip
            let mut trip: Vec<&Connection> = Vec::new();
            let mut cur = end_stop;
            while prev[cur] != None {
                trip.push(prev[cur].unwrap());
                cur = prev[cur].unwrap().dep_stop;
            }

            trip.reverse();

            return Some(TripResult {
                connections: trip
            });
        }

        // Important as we may have already found a better way
        if cost > dist[station] || !data.contains_key(&station) { continue; }

        // For each node we can reach, see if we can find a way with
        // a lower cost going through this node
        for (_, node) in data.get(&station).unwrap().neighbours.iter() {

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

// Finds the first connection in vec which is equal or greater than start_time
fn bin_search_arr<'a>(connections: &Vec<&'a Connection>, start_time: u32) -> Option<&'a Connection> {
    let mut left: usize = 0;
    let mut right: usize = connections.len() - 1;
    let mut ans = None;

    while left <= right {
        let mid = (left + right) / 2;

        if &connections[mid].dep_time < &start_time {
            left = mid + 1;
        } else {
            ans = Some(connections[mid]);

            if mid == 0 {
                break;
            }

            right = mid - 1;
        }

    };

    return ans;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bin_search() {
        let connections1 = vec![
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 0, arr_time: 0 },
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 5, arr_time: 0 },
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 10, arr_time: 0 },
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 15, arr_time: 0 },
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 20, arr_time: 0 },
        ];

        let connections2 = vec![
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 0, arr_time: 0 },
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 5, arr_time: 0 },
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 10, arr_time: 0 },
            &Connection { dep_stop: 0, arr_stop: 0, dep_time: 15, arr_time: 0 },
        ];

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

    #[test]
    fn route_test() {
        // Very simple test, this doesnt prove anything :(
        let connections = vec![
            Connection { dep_stop: 0, arr_stop: 1, dep_time: 1, arr_time: 4 },
            Connection { dep_stop: 1, arr_stop: 2, dep_time: 5, arr_time: 9 },
            Connection { dep_stop: 2, arr_stop: 3, dep_time: 10, arr_time: 14 },
            Connection { dep_stop: 3, arr_stop: 4, dep_time: 15, arr_time: 19 },
            Connection { dep_stop: 4, arr_stop: 5, dep_time: 20, arr_time: 25 },
        ];

        let data = prepare(&connections);
        let trip = compute(&data, 0, 5, 0);

        assert!(trip.is_some());
        assert_eq!(compute(&data, 0, 5, 0).unwrap(), TripResult {
            connections: connections.iter().collect::<Vec<&Connection>>()
        });
    }
}