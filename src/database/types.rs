use diesel_derive_enum::DbEnum;

#[derive(Debug, DbEnum, PartialEq, Eq)]
#[DieselType = "Enum"]
pub enum ServiceStopType {
    Departure,
    ShortIntermediate,
    Intermediate,
    Pass,
    Arrival
}