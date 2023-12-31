# types-of-ants

it's the types of ants. see [www.typesofants.org](www.typesofants.org).

## Development

### Start the developer database

1. Navigate to `projects/ant-data-farm` and run `docker-compose up -d`.

### Start the developer website

1. Navigate to `projects/ant-on-the-web/website` and run `npm dev`.

### Start the developer server

1. Navigate to `projects/ant-on-the-web/server` and run `cargo run`.

## Production

### Start the production database

1. Log onto the raspberry pi `pi@$(find_rpi)` if you're Kaspar and have that shell command. If not, find the IP yourself, smart guy.
1. Run `cd ~/projects/types-of-ants/projects/ant-data-farm` and `docker-compose up -d` to run the database.
1. Make sure it succeeds.

### Start tweeting

1. Log onto the raspberry pi `pi@$(find_rpi)`.
1. Make sure the database is up.
1. Run `cd ~/projects/types-of-ants/projects/ant-who-tweets` and run `cargo run &` to start the tweet job.
1. Using the PID from the `cargo run` (or find it with `jobs -l`) disown the process with `disown <pid>`.
