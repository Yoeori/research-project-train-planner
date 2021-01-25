use std::{collections::{BTreeSet, HashMap, HashSet}, ops::Range, cmp};

use crate::{benchable::{Benchable, BenchableLive}, types::{Connection, Timetable, Trip, TripPart, TripResult, TripUpdate}};

#[derive(Debug)]
struct Route<'a> {
    stops: Vec<usize>,
    trips: BTreeSet<&'a Trip>
}

impl<'a> Route<'a> {
    fn before(&self, p1: &usize, p2: &usize) -> bool {
        for stop in &self.stops {
            if stop == p1 {
                return true;
            } else if stop == p2 {
                return false;
            }
        }

        panic!("Stops not found in stops list!");
    }

    fn from(&self, p: &usize) -> Range<usize> {
        for (i, stop) in self.stops.iter().enumerate() {
            if stop == p {
                return i..(self.stops.len());
            }
        }

        panic!("Stop not found in stops list!");
    }

    fn trip_from(&self, si: usize, start_time: u32) -> Option<&Trip> {
        self.trips.iter().find(|t| t.connections[si].dep_time > start_time).map(|x| *x)
    }

    fn len(&self) -> usize {
        self.stops.len()-1
    }
}

const MAX_K: usize = 5;
const MAX_STATIONS: usize = 100000;

#[derive(Debug)]
pub struct RaptorBTree<'a> {
    routes: Vec<Route<'a>>,
    stops_routes: HashMap<usize, HashSet<usize>>,
    footpaths: HashMap<usize, HashMap<usize, u32>>,

    // For changes, we need to lookup the route it's a part of
    stops_route: HashMap<Vec<usize>, usize>
}

impl<'a> Benchable<'a> for RaptorBTree<'a> {

    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult> {
        let mut earliest_k_arrival: Vec<Vec<u32>> = vec![vec![u32::MAX - 3600 * 4; MAX_K]; MAX_STATIONS];
        earliest_k_arrival[dep_stop][0] = dep_time;

        let mut earliest_arrival:  Vec<u32> = vec![u32::MAX - 3600 * 4; MAX_STATIONS];
        earliest_arrival[dep_stop] = dep_time;

        // For constructing the journey
        let mut interchange: Vec<Option<(usize, usize, u32)>> = vec![None; MAX_STATIONS];
        let mut prev: Vec<Option<(&Connection, &Connection, (usize, usize, u32))>> = vec![None; MAX_STATIONS];

        let mut marked = HashSet::new();
        marked.insert(dep_stop);

        for k in 1..MAX_K {
            let mut q: HashMap<usize, usize> = HashMap::new();

            for p in &marked {
                for r in self.stops_routes.get(p).unwrap_or(&HashSet::new()) {
                    // Tracking: https://github.com/rust-lang/rfcs/blob/master/text/2497-if-let-chains.md :(
                    if let Some(p2) = q.get(r) {
                        if !self.routes[*r].before(p, p2) {
                            continue;
                        }
                    }
                    q.insert(*r, *p);
                }
            }

            marked.clear();

            for (&r, p) in q.iter() {
                let mut t: Option<&Trip> = None;
                let mut t_from: usize = 0;

                for (i, pi) in self.routes[r].from(p).map(|i| (i, self.routes[r].stops[i])) {
                    if !t.is_none() && t.unwrap().connections[i-1].arr_time < cmp::min(earliest_arrival[arr_stop], earliest_arrival[pi]) {
                        earliest_k_arrival[pi][k] = t.unwrap().connections[i-1].arr_time;
                        earliest_arrival[pi] = t.unwrap().connections[i-1].arr_time;
                        prev[pi] = Some((
                            &t.unwrap().connections[t_from],
                            &t.unwrap().connections[i-1],
                            interchange[t.unwrap().connections[t_from].dep_stop].unwrap()
                        ));
                        marked.insert(pi);
                    }

                    if i < self.routes[r].len() && (t.is_none() || 
                       earliest_k_arrival[pi][k-1] + self.footpaths.get(&pi).unwrap().get(&pi).unwrap() < t.unwrap().connections[i].dep_time) {
                        t = self.routes[r].trip_from(i, earliest_k_arrival[pi][k-1] + self.footpaths.get(&pi).unwrap().get(&pi).unwrap());
                        interchange[pi] = Some((pi, pi, *self.footpaths.get(&pi).unwrap().get(&pi).unwrap()));
                        t_from = i;
                    }
                }
            }

            // Look at footpaths
            for &p in marked.clone().iter() {
                for (&p2, &dur) in self.footpaths.get(&p).unwrap() {
                    if earliest_k_arrival[p][k] + dur < earliest_k_arrival[p2][k] {
                        earliest_k_arrival[p2][k] = earliest_k_arrival[p][k] + dur;
                        interchange[p2] = Some((p, p2, dur))
                    }
                    marked.insert(p2);
                }
            }

            if marked.is_empty() {
                break;
            }

        }

        let mut parts: Vec<TripPart> = Vec::new();
        let mut cur = arr_stop;
        while let Some((c1, c2, (p1, p2, dur))) = prev[cur] {
            parts.push(TripPart::Connection(c1, c2));
            parts.push(TripPart::Footpath(p1, p2, dur));
            cur = c1.dep_stop;
        }

        parts.reverse();

        if parts.is_empty(){
            return None;
        }
        
        parts.remove(0);

        return Some(TripResult {
            parts
        });
    }

    fn name(&self) -> &'static str {
        "RAPTOR with BTree"
    }

    fn new(timetable: &'a Timetable) -> Self where Self: Sized {

        let mut routes_map = HashMap::<Vec<usize>, BTreeSet<&Trip>>::new();

        fn trip_to_route(trip: &Trip) -> Vec<usize> {
            let mut res: Vec<usize> = trip.connections.iter().map(|c| c.dep_stop).collect();
            res.push(trip.connections.last().unwrap().arr_stop);
            res
        }

        for trip in &timetable.trips {
            let trip_route = trip_to_route(trip);
            if let Some(route) = routes_map.get_mut(&trip_route) {
                route.insert(trip);
            } else {
                routes_map.insert(trip_route, {
                    let mut set = BTreeSet::new();
                    set.insert(trip);
                    set
                });
            }
        }

        // We now have a list of routes and trips, and use this to build the data structure as
        // discussed in the appendix of the "Round-based public transit routing" paper
        // However, since we do not need the caching optimizations they are not put adjecant

        let mut routes: Vec<Route> = vec![];
        let mut stops_routes: HashMap<usize, HashSet<usize>> = HashMap::new();
        let mut stops_route: HashMap<Vec<usize>, usize> = HashMap::new();

        for (vec_route, trips) in routes_map.into_iter() {
            for stop in &vec_route {
                if let Some(stop_routes) = stops_routes.get_mut(stop) {
                    stop_routes.insert(routes.len());
                } else {
                    stops_routes.insert(*stop, {
                        let mut set = HashSet::new();
                        set.insert(routes.len());
                        set
                    });
                }
            }

            routes.push(Route {
                stops: vec_route.clone(),
                trips: trips.into_iter().collect(),
            });

            stops_route.insert(
                vec_route,
                routes.len()-1
            );
        }

        RaptorBTree {
            routes,
            stops_routes,
            footpaths: timetable.footpaths.clone().into_iter().map(|(p1, p2s)| (p1, p2s.into_iter().collect())).collect(),
            stops_route
        }
    }

}

impl<'a> BenchableLive<'a> for RaptorBTree<'a> {
    fn update(&mut self, update: &'a TripUpdate) {
        
        fn get_stops(trip: &Trip) -> Vec<usize> {
            let mut res = vec![trip.connections[0].dep_stop];
            for conn in &trip.connections {
                res.push(conn.arr_stop);
            }
            res
        }

        let mut delete_trip = None;
        let mut add_trip = None;

        match update {
            TripUpdate::DeleteTrip { trip } => {
                delete_trip = Some(trip);
            }
            TripUpdate::AddTrip { trip } => {
                add_trip = Some(trip);
            }
            TripUpdate::AddConnection { old_trip, new_trip, connection: _ } => {
                delete_trip = Some(old_trip);
                add_trip = Some(new_trip);
            }
            TripUpdate::DeleteConnection { old_trip, new_trip, connection: _ } => {
                delete_trip = Some(old_trip);
                add_trip = Some(new_trip);
            }
        }

        if let Some(trip) = delete_trip {
            let route = self.routes.get_mut(*self.stops_route.get(&get_stops(trip)).unwrap()).unwrap();
            route.trips.remove(trip);
        }

        if let Some(trip) = add_trip {
            let stops = get_stops(trip);
            if !self.stops_route.contains_key(&stops) {
                self.routes.push(Route {
                    stops: stops.clone(),
                    trips: BTreeSet::new()
                });
                self.stops_route.insert(stops.clone(), self.routes.len());
            }

            let route = self.routes.get_mut(*self.stops_route.get(&stops).unwrap()).unwrap();
            route.trips.insert(trip);
        }
    }
}

alg_test!(RaptorBTree);