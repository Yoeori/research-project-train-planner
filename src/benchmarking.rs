use crate::types::{Connection};
use crate::algorithms::{td, csa};

use std::error::Error;
use std::time::SystemTime;
use std::fs;
use std::fs::File;
use std::io::Read;

use flate2::read::GzDecoder;

// Currently this primarily compares TD and CSA.
// TODO implement for list of benchables

#[allow(dead_code)]
pub fn bench() -> Result<(), Box<dyn Error + 'static>> {
    println!("Reading data...");

    let mut data = GzDecoder::new(File::open("data/bench_data_48h.gz")?);
    let mut connections = String::new();
    data.read_to_string(&mut connections)?;

    let connections: Vec<Connection> = connections.split("\n").filter(|e| !e.is_empty()).map(|e| Connection::parse_from_string(e)).collect::<Result<Vec<_>, _>>()?;
    let stations = fs::read_to_string("data/stations").unwrap().split("\n").map(|e| e.parse::<usize>()).collect::<Result<Vec<usize>, _>>()?;

    println!("Finished reading data: {} connections", connections.len());

    compare(&connections, &stations)?;
    benchmark(&connections, &stations)?;

    Ok(())
}

fn compare(connections: &Vec<Connection>, stations: &Vec<usize>) -> Result<(), Box<dyn Error + 'static>> {
    let dijkstra_data = td::prepare(&connections);

    for &station in stations {
        let start_csa = SystemTime::now();
        let csa_res = csa::compute(&connections, station, *(stations.get(0).unwrap()), 0);
        let dur_csa = start_csa.elapsed();
        let start_dij = SystemTime::now();
        let dij_res = td::compute(&dijkstra_data, station, *(stations.get(0).unwrap()), 0);
        let dur_dij = start_dij.elapsed();

        if let Some(csa_res) = csa_res {
            if let Some(dij_res) = dij_res {
                println!("CSA: {} {:?} DIJKSTRA: {} {:?}", csa_res.arrival(), dur_csa, dij_res.arrival(), dur_dij)
            } else {
                println!("ERROR");
            }
        }
    }

    Ok(())
}

fn benchmark(connections: &Vec<Connection>, stations: &Vec<usize>) -> Result<(), Box<dyn Error + 'static>> {
    println!("Benchmarking CSA");
    let start = SystemTime::now();

    for &station in stations {
        if station == 3264 {
            println!("{:?}", csa::compute(&connections, station, *(stations.get(0).unwrap()), 0));
        }
        
        csa::compute(&connections, station, *(stations.get(0).unwrap()), 0);
    }
    println!("Duration CSA: {:?}", start.elapsed()?);

    println!("Benchmarking Dijkstra Advanced");
    let start_preparing = SystemTime::now();
    let dijkstra_data = td::prepare(&connections);
    println!("Duration preparing: {:?}", start_preparing.elapsed()?);

    let start = SystemTime::now();
    for &station in stations {
        if station == 3264 {
            println!("{:?}", td::compute(&dijkstra_data, station, *(stations.get(0).unwrap()), 0));
        }
        
        td::compute(&dijkstra_data, station, *(stations.get(0).unwrap()), 0);
    }
    println!("Duration Dijkstra: {:?}", start.elapsed()?);
    
    Ok(())
}