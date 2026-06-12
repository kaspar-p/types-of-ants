FROM postgres:17

COPY migrations/ /docker-entrypoint-initdb.d/
COPY seed-data/ /docker-entrypoint-initdb.d/
