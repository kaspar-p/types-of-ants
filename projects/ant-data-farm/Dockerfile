FROM postgres:15

# Copy all migration/schema files into the database
COPY data/sql/ /docker-entrypoint-initdb.d/
