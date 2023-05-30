mod db;
mod test;
mod tests;

use std::{sync::Arc, thread, time::Duration};

use crate::{db::Database, tests::ping::StatusData};
use tokio_cron_scheduler::{Job, JobScheduler};

#[tokio::main]
async fn main() -> Result<(), tokio_postgres::Error> {
    let database: Database = db::connect().await?;
    let arc_db = Arc::new(database);

    // Create the `CronJob` object.
    let schedule: JobScheduler = match JobScheduler::new().await {
        Ok(s) => s,
        Err(e) => panic!("Creating job scheduler failed {}", e),
    };

    let async_job = Job::new_async("1/5 * * * * * *", move |_, _| {
        let db = arc_db.clone();
        Box::pin(async move {
            on_cron(db).await;
        })
    })
    .expect("String parsing failed for cron schedule!");

    schedule
        .add(async_job)
        .await
        .expect("Creating cron job failed!");

    // Start the cronjob.
    println!("Creating cron job...");
    schedule
        .start()
        .await
        .expect("Starting cron scheduling failed!");

    loop {
        thread::sleep(Duration::from_millis(100));
    }
}

// Our cronjob handler.
async fn on_cron(database: Arc<Database>) -> () {
    println!("Testing...");

    // Collect all tests
    let tests = test::get_all_tests();

    let mut metrics: Vec<StatusData> = Vec::new();
    for test in tests {
        metrics.extend(test.await);
    }

    for metric in metrics.as_slice() {
        println!("{:?}", metric);
    }

    // TODO: Insert into the database here!
    let result = database.insert_status_data(metrics).await;
    match result {
        Err(e) => println!("Inserting data failed {}", e),
        Ok(_) => println!("Test iteration succeeded!"),
    }
}
