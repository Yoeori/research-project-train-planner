use crate::types::{Trip, TripUpdate};

pub trait Benchable {
    fn new(&mut self, trips: Vec<Trip>);
    fn update(&mut self, update: TripUpdate);
    fn find_earliest_arrival(&self, dep_stop: u32, arr_stop: u32, dep_time: usize);
}

pub trait BenchableProfile {
    // TODO
}

// See https://play.rust-lang.org/?gist=5cc19fdf03a6624e66a84488e72e26a4&version=stable for possible implentation of generalized test for multiple benchables.