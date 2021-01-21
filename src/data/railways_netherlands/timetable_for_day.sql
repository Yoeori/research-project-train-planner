SELECT service_stops.service_id, service_stops.type as "type_", service_stops.station_code, service_stops.arr_time, service_stops.dep_time, service_stops.platform
FROM services, validities, service_stops
WHERE services.validity_id = validities.id
AND validities.date = ?
AND service_stops.service_id = services.id
AND service_stops.type <> "pass"
ORDER BY service_id, ordering ASC