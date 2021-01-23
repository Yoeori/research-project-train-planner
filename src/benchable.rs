use std::{collections::HashSet, ops::Range};

use crate::types::{Timetable, TripResult, TripUpdate};

pub trait Benchable<'a> {
    fn new(timetable: &'a Timetable) -> Self where Self: Sized;
    fn name(&self) -> &'static str;
    fn find_earliest_arrival(&self, dep_stop: usize, arr_stop: usize, dep_time: u32) -> Option<TripResult>;
}

pub trait BenchableLive<'a>: Benchable<'a> {
    fn update(&mut self, update: &'a TripUpdate);
}

pub trait BenchableProfile<'a>: Benchable<'a> {
    fn find_earliest_arrival_profile_set(&self, dep_stop: usize, arr_stop: usize, range: Range<u32>) -> HashSet<TripResult>;
}