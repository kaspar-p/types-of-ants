mod db;
mod test;
mod tests;

use crate::db::Database;
use cronjob::CronJob;

const MODE: &str = "beta";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection_string = format!("postgresql://postgres:1234@localhost:5432/{}", MODE);
    let connection_pool = db::connect(connection_string);

    // Create the `CronJob` object.
    let mut cron: CronJob = CronJob::new("Test Cron", move |_name: &str| -> () {
        on_cron(&Database {
            connection: connection_pool.clone().get().unwrap(),
        });
    });

    // Run every 10 seconds
    cron.seconds("*/10");

    // Start the cronjob.
    cron.start_job();

    return Ok(());
}

// Our cronjob handler.
fn on_cron(database: &Database) -> () {
    // Collect all tests
    let tests = test::get_all_tests(database);
    tests();

    // Check if the tests are not yet in the DB
    // Put all tests into the DB, with UUID if they don't already have one
    // Call each test on a list of projects.

    // For each project, do every test
    // for url in urls {
    //     let res = client.get(url).send();
    //     emit_data(url, res.unwrap());
    // }
}
