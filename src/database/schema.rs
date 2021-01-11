table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    attributes (id) {
        id -> Varchar,
        description -> Nullable<Varchar>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    dvs_messages (id) {
        id -> Integer,
        created_at -> Datetime,
        envelope -> Nullable<Varchar>,
        message -> Nullable<Text>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    services (id) {
        id -> Unsigned<Integer>,
        validity_id -> Unsigned<Integer>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    service_attributes (service_id, attribute_id) {
        service_id -> Unsigned<Integer>,
        attribute_id -> Varchar,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    service_stops (service_id, ordering) {
        service_id -> Unsigned<Integer>,
        ordering -> Unsigned<Integer>,
        #[sql_name = "type"]
        type_ -> Enum,
        station_code -> Varchar,
        arr_time -> Nullable<Unsigned<Smallint>>,
        dep_time -> Nullable<Unsigned<Smallint>>,
        platform -> Nullable<Varchar>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    service_trns_modes (service_id, trns_mode_id) {
        service_id -> Unsigned<Integer>,
        trns_mode_id -> Varchar,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    stations (code) {
        code -> Varchar,
        name -> Varchar,
        country -> Varchar,
        timezone -> Tinyint,
        interchange_duration -> Unsigned<Tinyint>,
        is_interchange_station -> Bool,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    trns_modes (id) {
        id -> Varchar,
        description -> Nullable<Varchar>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::database::types::*;

    validities (id, date) {
        id -> Unsigned<Integer>,
        date -> Date,
    }
}

joinable!(service_attributes -> attributes (attribute_id));
joinable!(service_attributes -> services (service_id));
joinable!(service_stops -> services (service_id));
joinable!(service_stops -> stations (station_code));
joinable!(service_trns_modes -> services (service_id));
joinable!(service_trns_modes -> trns_modes (trns_mode_id));

allow_tables_to_appear_in_same_query!(
    attributes,
    dvs_messages,
    services,
    service_attributes,
    service_stops,
    service_trns_modes,
    stations,
    trns_modes,
    validities,
);
