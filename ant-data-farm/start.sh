# Create the volume
docker volume create pg-db

# Run the database, ready to accept connections
docker run --rm -P -p 5432:5432 -e POSTGRES_PASSWORD="1234" --name pg -v pg-db:/etc/db postgres

# Connect to the postgres docker image
pgcli postgresql://postgres:1234@localhost:5432/postgres