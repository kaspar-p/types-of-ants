mod test;
mod tests;

use crate::tests::ping::StatusData;
use ant_data_farm::{AntDataFarmClient, DatabaseConfig, DatabaseCredentials};
use std::sync::Arc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::error;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn get_config() -> Result<DatabaseConfig, dotenv::Error> {
    Ok(DatabaseConfig {
        creds: Some(DatabaseCredentials {
            database_name: dotenv::var("DB_PG_NAME")?,
            database_user: dotenv::var("DB_PG_USER")?,
            database_password: dotenv::var("DB_PG_PASSWORD")?,
        }),
        host: Some(dotenv::var("DB_HOST")?),
        port: Some(7000),
    })
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_file(true)
        .with_ansi(false)
        .with_writer(tracing_appender::rolling::hourly(
            "./logs/ant-just-checking-in",
            "ant-just-checking-in.log",
        ))
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let client = match AntDataFarmClient::new(Some(get_config().unwrap())).await {
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            error!("Ending CRON early!");
            return;
        }
        Ok(client) => Arc::new(client),
    };
    info!("Connected to database!");

    // Create the CronJob object.
    let schedule: JobScheduler = match JobScheduler::new().await {
        Ok(s) => s,
        Err(e) => panic!("Creating job scheduler failed {e}"),
    };

    let async_job = Job::new_async("*/5 * * * * * *", move |_, _| {
        let db = client.clone();
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
        info!("Sleeping 1800 seconds...");
        tokio::time::sleep(Duration::from_secs(1800)).await;
    }
}

// Our cronjob handler.
async fn on_cron(_database: Arc<AntDataFarmClient>) {
    println!("Testing...");

    // Collect all tests
    let tests = test::get_all_tests();

    let mut metrics: Vec<StatusData> = Vec::new();
    for test in tests {
        metrics.extend(test.await);
    }

    for metric in metrics.as_slice() {
        println!("{metric:?}");
    }

    // TODO: Insert into the database here!
    // let result = database.insert_status_data(metrics).await;
    // match result {
    //     Err(e) => println!("Inserting data failed {e}"),
    //     Ok(_) => println!("Test iteration succeeded!"),
    // }
}
