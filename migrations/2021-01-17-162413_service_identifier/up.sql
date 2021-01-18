CREATE TABLE service_identifier (
    service_id INT UNSIGNED NOT NULL,
    identifier INT UNSIGNED NOT NULL,
    from_index SMALLINT UNSIGNED NOT NULL,
    to_index SMALLINT UNSIGNED NOT NULL,
    PRIMARY KEY (service_id, identifier, from_index, to_index)
);