FROM postgres:15

# Copy all migration/schema files into the database
COPY migrations/ /docker-entrypoint-initdb.d/
