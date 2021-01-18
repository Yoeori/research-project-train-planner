use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{benchable::Benchable, types::{Connection, Timetable, Trip, TripResult}};

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

// Some helper types to make naming more clear
type Stop = usize;
type Route = HashMap<Stop, usize>;

#[derive(Debug)]
struct Raptor<'a> {
    timetable: &'a Timetable,
    data: HashMap<usize, Station<'a>>,
    routes: HashMap<usize, HashSet<Route>>
}

impl<'a> Benchable<'a> for Raptor<'a> {
    fn new(trips: &'a Timetable) -> Self where Self: Sized {
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

        fn trip_to_route(trip: &Trip) -> Route {
            let mut route: Route = trip.connections.iter().enumerate().map(|(i, conn)| (conn.dep_stop, i)).collect();
            route.insert(trip.connections.last().unwrap().arr_stop, route.len());
            route
        }

        let mut routes: HashMap<usize, HashSet<Route>> = HashMap::new();
        for trip in &trips.trips {
            let route = trip_to_route(trip);
            for &stop in route.keys() {
                routes.entry(stop).or_insert_with(|| HashSet::new()).insert(route);
            }
        }

        Raptor {
            timetable: trips,
            data: stations,
            routes: HashMap::new()
        }
    }

    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult> {
        let k_arrivals = {
            let mut k_connections: HashMap<usize, HashMap<u16, usize>> = HashMap::new();
            for &stop in self.data.keys() {
                k_connections.insert(stop, HashMap::new());
            }
            k_connections
        };

        let k_connections = {
            let mut k_arrivals: HashMap<u16, HashMap<usize, u32>> = HashMap::new();
            let mut initial_arrivals = HashMap::new();
            for &stop in self.data.keys() {
                initial_arrivals.insert(stop, u32::MAX);
            }

            initial_arrivals.insert(dep_stop, dep_time);

            k_arrivals.insert(0, initial_arrivals);
            k_arrivals
        };

        let mut marked_stops: HashSet<usize> = HashSet::new();
        marked_stops.insert(dep_stop);

        for i in 1.. {
            

            // empty Q??
            // let queue = HashSet::new();
            // for stop in &marked_stops {
            //     // Get routes serving stop
            //     for route in self.data.get(&stop) {

            //     }
            // }

            for stop in &marked_stops {
                // Footpaths
            }

            if marked_stops.is_empty() {
                break;
            }
        }


        todo!()
    }
}