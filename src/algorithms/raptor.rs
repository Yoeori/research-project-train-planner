use std::{collections::{BTreeSet, HashMap, HashSet}, ops::Range, cmp};

use crate::{benchable::{Benchable}, types::{Connection, Timetable, Trip, TripPart, TripResult}};

#[derive(Debug)]
struct Route<'a> {
    stops: Vec<usize>,
    trips: Vec<&'a Trip>
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
        let mut left: usize = 0;
        let mut right: usize = self.trips.len() - 1;
        let mut ans = None;
        while left <= right {
            let mid = (left + right) / 2;

            if &self.trips[mid].connections[si].dep_time < &start_time {
                left = mid + 1;
            } else {
                ans = Some(self.trips[mid]);

                if mid == 0 {
                    break;
                }

                right = mid - 1;
            }

        };

        ans
    }

    fn len(&self) -> usize {
        self.trips[0].connections.len()
    }
}

const MAX_K: usize = 5;
const MAX_STATIONS: usize = 100000;

#[derive(Debug)]
pub struct Raptor<'a> {
    routes: Vec<Route<'a>>,
    stops_routes: HashMap<usize, HashSet<usize>>,
    footpaths: HashMap<usize, HashMap<usize, u32>>
}

impl<'a> Benchable<'a> for Raptor<'a> {

    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult> {
        let mut earliest_k_arrival: Vec<Vec<u32>> = vec![vec![u32::MAX - 3600 * 4; MAX_K]; MAX_STATIONS];
        earliest_k_arrival[dep_stop][0] = dep_time;

        let mut earliest_arrival:  Vec<u32> = vec![u32::MAX - 3600 * 4; MAX_STATIONS];
        earliest_arrival[dep_stop] = dep_time;

        // For constructing the journey
        let mut interchange: Vec<Option<(usize, usize, u32)>> = vec![None; MAX_STATIONS * 10];
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
                        prev[pi] = Some((&t.unwrap().connections[t_from], &t.unwrap().connections[i-1], interchange[t.unwrap().connections[t_from].dep_stop].unwrap()));
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
        "RAPTOR with Vec"
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
                stops: vec_route,
                trips: trips.into_iter().collect(),
            });
        }

        Raptor {
            routes,
            stops_routes,
            footpaths: timetable.footpaths.clone().into_iter().map(|(p1, p2s)| (p1, p2s.into_iter().collect())).collect()
        }
    }

}

alg_test!(Raptor);