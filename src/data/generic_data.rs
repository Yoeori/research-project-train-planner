use std::{collections::HashMap, error::Error, fs::{self, File}, io::Read};

use flate2::read::GzDecoder;

use crate::types::{Connection, Stop, Timetable, Trip};

#[derive(Debug)]
struct SimpleStop {
    id: usize
}

impl Stop for SimpleStop {
    fn to_string(&self) -> String {
        format!("{}", self.id)
    }

    fn coords(&self) -> Option<(f64, f64)> {
        None
    }

    fn distance(&self, _: &Box<dyn Stop>) -> Option<f64> {
        None
    }
}

pub fn get_data() -> Result<Timetable, Box<dyn Error>> {
    let mut data = GzDecoder::new(File::open("data/bench_data_48h.gz")?);
    let mut connections = String::new();
    data.read_to_string(&mut connections)?;

    let trips: Vec<Trip> = connections.split("\n")
        .filter(|e| !e.is_empty())
        .map(|e| Connection::parse_from_string(e, 0))
        .enumerate().map(|(identifier, c)| c.map(move |conn| Trip {
            identifier,
            connections: vec![conn]
        }))
        .collect::<Result<Vec<_>, _>>()?;

    let stops = fs::read_to_string("data/stations").unwrap().split("\n")
        .map(|e| SimpleStop { id: e.parse::<usize>().unwrap() })
        .map(|e| (e.id, Box::new(e) as Box<dyn Stop>))
        .collect::<HashMap<usize, Box<dyn Stop>>>();

    Ok(Timetable {
        trips,
        stops,
        footpaths: HashMap::new()
    })
}