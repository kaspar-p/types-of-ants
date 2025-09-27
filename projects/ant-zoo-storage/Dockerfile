FROM postgres:17

ARG TYPESOFANTS_ENV

# Copy all migration/schema files into the database
COPY migrations/ /docker-entrypoint-initdb.d/
COPY seed-data/ /docker-entrypoint-initdb.d/
COPY seed-data/${TYPESOFANTS_ENV}/ /docker-entrypoint-initdb.d/
