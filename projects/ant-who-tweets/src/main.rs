use ant_library::set_global_logs;
use ant_who_tweets::{ant_client, cron_tweet, get_config};
use chrono::Timelike;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

#[tokio::main]
async fn main() {
    set_global_logs("ant-who-tweets");

    let scheduler = JobScheduler::new().await.unwrap();

    // Midnight UTC is 8pm EST is 6pm MST
    let utc_hour_to_tweet = 0;

    info!(
        "Starting up! Local hour: {}, hour to tweet: {}",
        chrono::offset::Local::now().hour(),
        utc_hour_to_tweet
    );

    let config = get_config().expect("Getting environment variables failed!");
    info!("Tweeting on behalf of @{}", config.twitter.handle);

    // Check the connection to the database by creating a dummy "test" client at the front.
    ant_client(config.database).await;

    scheduler
        .add(
            // 6pm MST is midnight UTC
            Job::new_async(
                format!("0 0 {} * * *", utc_hour_to_tweet).as_str(),
                |_, __| {
                    Box::pin(async move {
                        info!("Entering cron_tweet()...");
                        cron_tweet(get_config().expect("environment"))
                            .await
                            .expect("Cron tweet failed!");
                    })
                },
            )
            .unwrap(),
        )
        .await
        .unwrap();

    // Start the scheduler
    scheduler.start().await.unwrap();

    loop {
        info!("Sleeping 1800 seconds...");
        tokio::time::sleep(Duration::from_secs(1800)).await;
    }
}
