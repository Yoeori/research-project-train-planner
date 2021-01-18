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

#[allow(dead_code)]
pub fn get_data() -> Result<Timetable, Box<dyn Error>> {
    let mut data = GzDecoder::new(File::open("data/bench_data_48h.gz")?);
    let mut connections = String::new();
    data.read_to_string(&mut connections)?;

    let connections: Vec<Connection> = connections.split("\n").filter(|e| !e.is_empty()).map(|e| Connection::parse_from_string(e)).collect::<Result<Vec<_>, _>>()?;
    let stops = fs::read_to_string("data/stations").unwrap().split("\n")
        .map(|e| SimpleStop { id: e.parse::<usize>().unwrap() })
        .map(|e| (e.id, Box::new(e) as Box<dyn Stop>))
        .collect::<HashMap<usize, Box<dyn Stop>>>();

    Ok(Timetable {
        trips: vec![Trip {
            identifier: 0,
            connections
        }],
        stops
    })
}