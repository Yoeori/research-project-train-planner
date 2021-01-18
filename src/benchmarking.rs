use std::{error::Error, time::Instant};

use csv::Writer;
use serde::Serialize;

use crate::{algorithms, benchable::{Benchable, BenchableLive}, types::{Timetable, TripUpdate}};

#[derive(Serialize, Debug)]
struct RouteBench {
    data_set: String,
    algorithm: String,
    live: bool,
    distance: Option<f64>,
    time_in_ns: u128
}

#[derive(Serialize, Debug)]
struct UpdateBench {
    data_set: String,
    algorithm: String,
    time_in_ns: u128
}

pub fn bench_algorithms(data_set: &str, timetable: &Timetable) -> Result<(), Box<dyn Error>> {
    let mut res = vec![];
    
    for algorithm in algorithms::algorithms() {
        let benchable = algorithm(&timetable);
        println!("Benching: {}", benchable.name());
        res.extend(bench_algorithm(data_set, &benchable, &timetable));
    }

    let mut csv = Writer::from_path("bench.csv")?;
    res.iter().map(|record| {
        csv.serialize(record)
    }).collect::<Result<_, _>>()?;
    csv.flush()?;

    Ok(())
}

fn bench_algorithm<'a>(data_set: &str, benchable: &Box<dyn Benchable<'a> + 'a>, timetable: &'a Timetable) -> Vec<RouteBench> {
    let mut times = vec![];

    for (&stop1, place1) in &timetable.stops {
        for (&stop2, place2) in &timetable.stops {
            let before = Instant::now();
            benchable.find_earliest_arrival(stop1, stop2, 120000);
            let time = before.elapsed();
            times.push(RouteBench {
                data_set: data_set.to_string(),
                algorithm: benchable.name().to_string(),
                live: false,
                distance: place1.distance(&place2),
                time_in_ns: time.as_nanos(),
            });
        }
    }

    times
}

pub fn bench_algorithms_live<'a>(data_set: &str, timetable: &'a Timetable, updates: &'a Vec<TripUpdate>) -> Result<(), Box<dyn Error>> {
    let mut res_routes: Vec<RouteBench> = Vec::with_capacity(timetable.stops.len().pow(2));
    let mut res_updates: Vec<UpdateBench> = Vec::with_capacity(timetable.stops.len().pow(2));

    for algorithm in algorithms::algorithms_live() {
        let mut benchable = algorithm(&timetable);
        println!("Benching: {}", benchable.name());
        let (route_times, update_times) = bench_algorithm_live(data_set, &mut benchable, &timetable, updates);

        res_routes.extend(route_times);
        res_updates.extend(update_times);
    }

    let mut csv = Writer::from_path("bench_live.csv")?;
    res_routes.iter().map(|record| {
        csv.serialize(record)
    }).collect::<Result<_, _>>()?;
    csv.flush()?;

    let mut csv = Writer::from_path("bench_live_updates.csv")?;
    res_updates.iter().map(|record| {
        csv.serialize(record)
    }).collect::<Result<_, _>>()?;
    csv.flush()?;

    Ok(())
}

fn bench_algorithm_live<'a>(data_set: &str, benchable: &mut Box<dyn BenchableLive<'a> + 'a>, timetable: &'a Timetable, updates: &'a Vec<TripUpdate>) -> (Vec<RouteBench>, Vec<UpdateBench>) {
    let mut route_times = vec![];
    let mut update_times = vec![];
    
    let updates_to_perform_per_iteration = (updates.len() / timetable.stops.len().pow(2)) + 1;
    let mut updates = updates.iter();

    for (&stop1, place1) in &timetable.stops {
        for (&stop2, place2) in &timetable.stops {
            let before = Instant::now();
            benchable.find_earliest_arrival(stop1, stop2, 120000);
            let time = before.elapsed();

            route_times.push(RouteBench {
                live: true,
                data_set: data_set.to_string(),
                algorithm: benchable.name().to_string(),
                distance: place1.distance(place2),
                time_in_ns: time.as_nanos(), 
            });

            for _ in 0..updates_to_perform_per_iteration {
                if let Some(update) = updates.next() {
                    let before = Instant::now();
                    benchable.update(&update);
                    let time = before.elapsed();
                    update_times.push(UpdateBench {
                        data_set: data_set.to_string(),
                        algorithm: benchable.name().to_string(),
                        time_in_ns: time.as_nanos(),
                    });
                }
            }
        }
    }

    (route_times, update_times)
}