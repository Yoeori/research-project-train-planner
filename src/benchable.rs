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
    // TODO
}

// See https://play.rust-lang.org/?gist=5cc19fdf03a6624e66a84488e72e26a4&version=stable for possible implentation of generalized test for multiple benchables.