# types-of-ants
it's the types of ants. see www.typesofants.org

## Start tweeting

1. Log onto the raspberry pi `pi@$(find_rpi)`
1. Go to `ant-data-farm` and run `docker-compose up -d` to run the database
2. Go to `ant-who-tweets` and run `cargo run &` to start the tweet job.
3. Using the PID from the `cargo run` (or find it with `jobs -l`) disown the process with `disown <pid>`
