-- SELECT DISTINCT service_stops.station_code, service_stops.platform
-- FROM service_stops
-- WHERE service_stops.type != 'pass'
-- ORDER BY station_code

SELECT code, lat, lng
FROM stations