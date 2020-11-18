mod csa;
mod dijkstra;

use flate2::read::GzDecoder;
use std::error::Error;
use std::fs::File;
use std::fs;
use std::io::Read;
use std::time::SystemTime;
use std::cmp::Ordering;

pub const MAX_STATIONS: usize = 100000;

#[derive(Debug, PartialEq, Eq)]
pub struct Connection {
    dep_stop: usize,
    arr_stop: usize,
    
    dep_time: u32,
    arr_time: u32
}

impl Ord for Connection {
    fn cmp(&self, other: &Connection) -> Ordering {
        self.dep_time.cmp(&other.dep_time)
            .then_with(|| self.arr_time.cmp(&other.arr_time))
            .then_with(|| self.dep_stop.cmp(&other.dep_stop))
            .then_with(|| self.arr_stop.cmp(&other.arr_stop))
    }
}

impl PartialOrd for Connection {
    fn partial_cmp(&self, other: &Connection) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Connection {
    fn parse(line: &str) -> Result<Connection, Box<dyn Error + 'static>> {
        // println!("{}", line);
        let mut splitted = line.split(" ").map(|t| { t.parse::<u32>() });

        Ok(Connection {
            dep_stop: splitted.next().ok_or("Missing dep_stop")?? as usize,
            arr_stop: splitted.next().ok_or("Missing arr_stop")?? as usize,
            dep_time: splitted.next().ok_or("Missing dep_time")??,
            arr_time: splitted.next().ok_or("Missing arr_time")??,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Trip<'a> {
    connections: Vec<&'a Connection>
}

impl Trip<'_> {

    fn arrival(&self) -> u32 {
        self.connections.last().unwrap().arr_time
    }

}

fn main() -> Result<(), Box<dyn Error + 'static>> {

    // Read bench data and convert to connections
    println!("Reading data...");

    let mut data = GzDecoder::new(File::open("data/bench_data_48h.gz")?);
    let mut connections = String::new();
    data.read_to_string(&mut connections)?;

    let connections: Vec<Connection> = connections.split("\n").filter(|e| !e.is_empty()).map(|e| Connection::parse(e)).collect::<Result<Vec<_>, _>>()?;
    let stations = fs::read_to_string("data/stations").unwrap().split("\n").map(|e| e.parse::<usize>()).collect::<Result<Vec<usize>, _>>()?;

    println!("Finished reading data: {} connections", connections.len());

    compare(&connections, &stations)?;
    return benchmark(&connections, &stations);
}

fn compare(connections: &Vec<Connection>, stations: &Vec<usize>) -> Result<(), Box<dyn Error + 'static>> {
    let dijkstra_data = dijkstra::prepare(&connections);

    for &station in stations {
        let start_csa = SystemTime::now();
        let csa_res = csa::compute(&connections, station, *(stations.get(0).unwrap()), 0);
        let dur_csa = start_csa.elapsed();
        let start_dij = SystemTime::now();
        let dij_res = dijkstra::compute(&dijkstra_data, station, *(stations.get(0).unwrap()), 0);
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
    let dijkstra_data = dijkstra::prepare(&connections);
    println!("Duration preparing: {:?}", start_preparing.elapsed()?);

    let start = SystemTime::now();
    for &station in stations {
        if station == 3264 {
            println!("{:?}", dijkstra::compute(&dijkstra_data, station, *(stations.get(0).unwrap()), 0));
        }
        
        dijkstra::compute(&dijkstra_data, station, *(stations.get(0).unwrap()), 0);
    }
    println!("Duration Dijkstra: {:?}", start.elapsed()?);
    
    Ok(())
}