-- Stations
CREATE TABLE stations (
    code VARCHAR(7) PRIMARY KEY NOT NULL,
    name VARCHAR(30) NOT NULL,
    country VARCHAR(4) NOT NULL,
    timezone TINYINT NOT NULL,
    interchange_duration TINYINT UNSIGNED NOT NULL,
    is_interchange_station BOOL NOT NULL
);

-- trns modes
CREATE TABLE trns_modes (
    id VARCHAR(5) PRIMARY KEY NOT NULL,
    description VARCHAR(30)
);

-- attributes
CREATE TABLE attributes (
    id VARCHAR(5) PRIMARY KEY NOT NULL,
    description VARCHAR(30)
);

-- Validity
CREATE TABLE validities (
    id INT UNSIGNED NOT NULL,
    date DATE NOT NULL,
    PRIMARY KEY (id, date)
);

-- Services
CREATE TABLE services (
    id INT UNSIGNED PRIMARY KEY NOT NULL,
    validity_id INT UNSIGNED NOT NULL,
    FOREIGN KEY (validity_id) REFERENCES validities(id)
);

CREATE TABLE service_stops (
    service_id INT UNSIGNED NOT NULL,
    ordering INT UNSIGNED NOT NULL,
    type ENUM('departure', 'short_intermediate', 'intermediate', 'pass', 'arrival') NOT NULL,
    station_code VARCHAR(7) NOT NULL,
    arr_time SMALLINT UNSIGNED,
    dep_time SMALLINT UNSIGNED,
    platform VARCHAR(3),

    PRIMARY KEY (service_id, ordering),
    FOREIGN KEY (service_id) REFERENCES services(id),
    FOREIGN KEY (station_code) REFERENCES stations(code)
);

CREATE TABLE service_trns_modes (
    service_id INT UNSIGNED NOT NULL,
    trns_mode_id VARCHAR(5) NOT NULL,

    PRIMARY KEY (service_id, trns_mode_id),
    FOREIGN KEY (service_id) REFERENCES services(id),
    FOREIGN KEY (trns_mode_id) REFERENCES trns_modes(id)
);

CREATE TABLE service_attributes (
    service_id INT UNSIGNED NOT NULL,
    attribute_id VARCHAR(5) NOT NULL,

    PRIMARY KEY (service_id, attribute_id),
    FOREIGN KEY (service_id) REFERENCES services(id),
    FOREIGN KEY (attribute_id) REFERENCES attributes(id)
);