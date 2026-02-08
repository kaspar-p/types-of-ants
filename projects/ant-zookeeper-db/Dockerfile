FROM postgres:17

# Copy all migration/schema files into the database
COPY migrations/ /docker-entrypoint-initdb.d/
COPY seed-data/ /docker-entrypoint-initdb.d/
