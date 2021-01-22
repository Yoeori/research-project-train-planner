use std::collections::HashMap;

use serde::{Deserialize};
use serde::de::{self, Deserializer, Unexpected};
use chrono::{DateTime, Local, NaiveDate};

use crate::types::{Connection, Trip};

#[derive(Deserialize, Debug)]
pub struct RITMessage {
    #[serde(rename = "ReisInformatieProductRitInfo")]
    pub message: TravelInformationRITInfo
}

#[derive(Deserialize, Debug)]
pub struct TravelInformationRITInfo {
    #[serde(rename = "RIPAdministratie")]
    pub administration: RIPAdministration,

    #[serde(rename = "RitInfo")]
    pub rit: RITInformation
}
#[derive(Deserialize, Debug)]
pub struct RIPAdministration {

}

#[derive(Deserialize, Debug)]
pub struct RITInformation {
    #[serde(rename = "TreinNummer")]
    pub trip_id: usize,

    #[serde(rename = "TreinDatum", deserialize_with = "chrono_date")]
    pub date: NaiveDate,

    // // Optional elements
    // #[serde(rename = "TreinNaam")]
    // pub name: Option<String>,

    // #[serde(rename = "TreinSoort")]
    // pub type_: Option<RITTrainType>,

    // #[serde(rename = "Vervoerder")]
    // pub company: String,

    // // Boolean train information
    // #[serde(rename = "SpeciaalKaartje", deserialize_with = "bool_from_j_n")]
    // pub special_ticket: bool,

    // #[serde(rename = "Reserveren", deserialize_with = "bool_from_j_n")]
    // pub reservation: bool,

    // #[serde(rename = "Toeslag", deserialize_with = "bool_from_j_n")]
    // pub surcharge: bool,

    #[serde(rename = "LogischeRit")]
    pub trip: RITLogical
}

#[derive(Debug, Deserialize)]
pub struct RITTrainType {
    #[serde(rename = "Code")]
    code: String,

    #[serde(rename = "$value")]
    name: String
}
#[derive(Debug, Deserialize)]
pub struct RITLogical {
    // This rit can be multiple rits eg: 5237_6439
    #[serde(rename = "LogischeRitNummer")]
    pub trip_id: String,

    #[serde(rename = "LogischeRitDeel", default = "Vec::new")]
    pub parts: Vec<RITLogicalPart>
}

#[derive(Debug, Deserialize)]
pub struct RITLogicalPart {
    #[serde(rename = "LogischeRitDeelNummer")]
    pub trip_id: usize,

    #[serde(rename = "LogischeRitDeelStation", default = "Vec::new")]
    pub stops: Vec<RITLogicalPartStop>
}

impl RITLogicalPart {
    pub fn to_trip(&self, stops: &HashMap<String, usize>) -> Option<Trip> {
        let mut connections = vec![];

        let mut prev_saved_stop: Option<&RITLogicalPartStop> = None;
        for next_stop in self.stops.iter() {
            if next_stop.stopping.iter().find(|s| s.state == RITState::Current)?.stopping {
                if let Some(prev_stop) = prev_saved_stop {
                    connections.push(Connection {
                        dep_stop: *stops.get(&prev_stop.station.code.to_lowercase())?,
                        arr_stop: *stops.get(&next_stop.station.code.to_lowercase())?,

                        dep_time: prev_stop.dep_time.iter().find(|s| s.state == RITState::Current)?.date.timestamp() as u32,
                        arr_time: next_stop.arr_time.iter().find(|s| s.state == RITState::Current)?.date.timestamp() as u32,

                        trip_id: self.trip_id
                    });
                    prev_saved_stop = Some(next_stop);
                } else {
                    prev_saved_stop = Some(next_stop);
                }
            }
        }

        Some(Trip {
            identifier: self.trip_id,
            connections
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct RITLogicalPartStop {
    #[serde(rename = "Station")]
    pub station: RITStation,

    #[serde(rename = "AankomstTijd", default = "Vec::new")]
    pub arr_time: Vec<RITDateTime>,

    #[serde(rename = "VertrekTijd", default = "Vec::new")]
    pub dep_time: Vec<RITDateTime>,

    // #[serde(rename = "TreinAankomstSpoor", default = "Vec::new")]
    // pub arr_stop: Vec<RITTrack>,

    // #[serde(rename = "TreinVertrekSpoor", default = "Vec::new")]
    // pub dep_stop: Vec<RITTrack>,

    #[serde(rename = "Stopt", default = "Vec::new")]
    pub stopping: Vec<RITStopping>
    
}

#[derive(Deserialize, Debug)]
pub  struct RITStation {
    #[serde(rename = "StationCode")]
    pub code: String,
    
    // #[serde(rename = "Type")]
    // pub station_type: u8,
    
    // #[serde(rename = "KorteNaam")]
    // pub short_name: String,
    
    // #[serde(rename = "MiddelNaam")]
    // pub middle_name: String,
    
    // #[serde(rename = "LangeNaam")]
    // pub long_name: String,
    
    // #[serde(rename = "UICCode")]
    // pub uic_code: u32
}

#[derive(Debug, Deserialize)]
pub struct RITDateTime {
    #[serde(rename = "InfoStatus", deserialize_with = "dvs_state")]
    pub state: RITState,

    #[serde(rename = "$value")]
    pub date: DateTime<Local>
}

#[derive(Debug, Deserialize)]
pub struct RITTrack {
    #[serde(rename = "InfoStatus", deserialize_with = "dvs_state")]
    pub state: RITState,

    #[serde(rename = "SpoorNummer")]
    pub track: Option<String>,

    #[serde(rename = "SpoorFase")]
    pub part: Option<String>
}

#[derive(Debug, Deserialize)]
pub struct RITStopping {
    #[serde(rename = "InfoStatus", deserialize_with = "dvs_state")]
    pub state: RITState,

    #[serde(rename = "$value", deserialize_with = "bool_from_j_n")]
    pub stopping: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RITState {
    Planned, Current
}

fn bool_from_j_n<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match char::deserialize(deserializer)? {
        'J' => Ok(true),
        'N' => Ok(false),
        other => Err(de::Error::invalid_value(
            Unexpected::Char(other),
            &"J or N",
        )),
    }
}

fn chrono_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let date = &String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| de::Error::invalid_value(
        Unexpected::Str(date),
        &"Date in the format YEAR-MO-DA",
    ))
}

fn dvs_state<'de, D>(deserializer: D) -> Result<RITState, D::Error>
where
    D: Deserializer<'de>,
{
    match &String::deserialize(deserializer)?[..] {
        "Gepland" => Ok(RITState::Planned),
        "Actueel" => Ok(RITState::Current),
        state => Err(de::Error::invalid_value(
            Unexpected::Str(state),
            &"State is either 'Actueel' or 'Gepland'",
        ))
    }
}