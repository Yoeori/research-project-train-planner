use serde::{Deserialize};
use serde::de::{self, Deserializer, Unexpected};
use serde_repr::{Serialize_repr, Deserialize_repr};
use chrono::{DateTime, Local, NaiveDate};

#[derive(Deserialize, Debug)]
pub struct DVSMessage {
    #[serde(rename = "ReisInformatieProductDVS")]
    pub message: TravelInformationMessageDVS
}

#[derive(Deserialize, Debug)]
pub struct TravelInformationMessageDVS {
    #[serde(rename = "DynamischeVertrekStaat")]
    pub dvs: DVS
}

#[derive(Deserialize, Debug)]
pub struct DVS {
    #[serde(rename = "RitId")]
    pub trip_id: usize,

    #[serde(rename = "RitDatum", deserialize_with = "chrono_date")]
    pub date: NaiveDate,

    #[serde(rename = "RitStation")]
    pub station: DVSStation,

    #[serde(rename = "Trein")]
    pub train: DVSTrain
}

#[derive(Deserialize, Debug)]
pub struct DVSTrain {
    #[serde(rename = "TreinNummer")]
    pub trip_id: usize,

    // Optional elements
    #[serde(rename = "TreinNaam")]
    pub name: Option<String>,

    #[serde(rename = "TreinSoort")]
    pub type_: Option<DVSTrainType>,

    #[serde(rename = "TreinFormule")]
    pub formula: Option<String>,


    // Boolean train information
    #[serde(rename = "SpeciaalKaartje", deserialize_with = "bool_from_j_n")]
    pub special_ticket: bool,

    #[serde(rename = "Reserveren", deserialize_with = "bool_from_j_n")]
    pub reservation: bool,

    #[serde(rename = "Toeslag", deserialize_with = "bool_from_j_n")]
    pub surcharge: bool,

    #[serde(rename = "RangeerBeweging", deserialize_with = "bool_from_j_n")]
    pub shunting: bool,

    #[serde(rename = "NietInstappen", deserialize_with = "optionable_bool_from_j_n")]
    pub boarding: Option<bool>,

    #[serde(rename = "AchterBlijvenAchtersteTreinDeel", deserialize_with = "bool_from_j_n")]
    pub rear_will_end: bool,

    #[serde(rename = "VertrekTijd")]
    pub dep_time: Vec<DVSDateTime>,

    #[serde(rename = "TreinVertrekSpoor")]
    #[serde(default = "Vec::new")]
    pub dep_track: Vec<DVSTrack>,

    #[serde(rename = "Wijziging")]
    #[serde(default = "Vec::new")]
    pub changes: Vec<DVSChange>,

    #[serde(rename = "TreinVleugel")]
    #[serde(default = "Vec::new")]
    pub sections: Vec<DVSTrainSection>
}

#[derive(Deserialize, Debug)]
pub struct DVSTrainSection {
    #[serde(rename = "TreinVleugelVertrekSpoor")]
    #[serde(default = "Vec::new")]
    pub dep_track: Vec<DVSTrack>,
    
    #[serde(rename = "StopStations")]
    #[serde(default = "Vec::new")]
    pub stations: Vec<DVSTrainSectionStations>,

    #[serde(rename = "Wijziging")]
    #[serde(default = "Vec::new")]
    pub changes: Vec<DVSChange>,
}

#[derive(Debug, Deserialize)]
pub struct DVSTrainSectionStations {
    #[serde(rename = "InfoStatus", deserialize_with = "dvs_state")]
    state: DVSState,

    #[serde(rename = "Station")]
    #[serde(default = "Vec::new")]
    stations: Vec<DVSStation>
}

#[derive(Deserialize, Debug)]
pub struct DVSChange {
    #[serde(rename = "WijzigingType")]
    pub change_type: u8
}

#[derive(Deserialize, Debug)]
pub  struct DVSStation {
    #[serde(rename = "StationCode")]
    pub code: String,
    
    #[serde(rename = "Type")]
    pub station_type: u8,
    
    #[serde(rename = "KorteNaam")]
    pub short_name: String,
    
    #[serde(rename = "MiddelNaam")]
    pub middle_name: String,
    
    #[serde(rename = "LangeNaam")]
    pub long_name: String,
    
    #[serde(rename = "UICCode")]
    pub uic_code: u32
}

#[derive(Debug, Deserialize)]
pub struct DVSDateTime {
    #[serde(rename = "InfoStatus", deserialize_with = "dvs_state")]
    state: DVSState,

    #[serde(rename = "$value")]
    date: DateTime<Local>
}

#[derive(Debug, Deserialize)]
pub struct DVSTrack {
    #[serde(rename = "InfoStatus", deserialize_with = "dvs_state")]
    state: DVSState,

    #[serde(rename = "SpoorNummer")]
    track_number: u8,

    #[serde(rename = "SpoorFase")]
    track_part: Option<char>
}

#[derive(Debug, Deserialize)]
pub struct DVSTrainType {
    #[serde(rename = "Code")]
    code: String,

    #[serde(rename = "$value")]
    name: String
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum DVSUpdateType {
    DepartDelay = 10,
    DepartStopChange = 20,
    DepartStopFixation = 22,

    StopChange = 30,

    ExtraTrainOrDepart = 31,
    CancelledTrainOrDepart = 32,

    Diversion = 33,
    TripShortening = 34,
    TripExtension = 35,

    TrainStateChange = 40,
    TripDiversion = 41,
    
    NoLiveInformation = 50,
    TrainReplacingTransport = 51
}

#[derive(Debug)]
pub enum DVSState {
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

fn optionable_bool_from_j_n<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(bool_from_j_n(deserializer).ok())
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

fn dvs_state<'de, D>(deserializer: D) -> Result<DVSState, D::Error>
where
    D: Deserializer<'de>,
{
    match &String::deserialize(deserializer)?[..] {
        "Gepland" => Ok(DVSState::Planned),
        "Actueel" => Ok(DVSState::Current),
        state => Err(de::Error::invalid_value(
            Unexpected::Str(state),
            &"State is either 'Actueel' or 'Gepland'",
        ))
    }
}