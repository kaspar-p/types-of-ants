# types of ants

it's the types of ants. see [www.typesofants.org](www.typesofants.org).

## Development

To learn about the architecture and how everything works, see the
[developer guide documents](./developer-guide) here.

## Production

### Start the production database

1. Log onto the host running the database, likely `antworker000`.
1. Run `cd ~/types-of-ants/projects/ant-data-farm` and
   `docker-compose up -d ant-data-farm` to run the database.
1. Make sure it succeeds.
1. Connect via `psql -p 7000 -h 0.0.0.0 -U typesofants` and type in the password
   in the `.env` file on this machine, make sure it's healthy.

### Start tweeting

1. Log onto the host running the tweet job, I think it's
   `antworker000.hosts.typesofants.org` with `ssh2ant 000`.
1. Make sure the database is up.
1. Run `cd ~/types-of-ants/projects/ant-who-tweets` and run
   `nohup cargo run & disown` to start the tweet job.

### Start the host management webserver

1. Log onto the host `ssh2ant <hostnum>`
1. Run `cd ~/types-of-ants/projects/ant-host-agent`
1. Make sure there is a `.env` with `HOST_AGENT_PORT` defined to a port number,
   likely 4499.
1. Run the program with `nohup cargo run & disown` to make sure it does not get
   killed when you exit.
