use diesel_derive_enum::DbEnum;

#[derive(Debug, DbEnum)]
#[DieselType = "Enum"]
pub enum ServiceStopType {
    Departure,
    ShortIntermediate,
    Intermediate,
    Pass,
    Arrival
}