use ant_data_farm::ants::Tweeted;
use ant_data_farm::DaoTrait;
use ant_data_farm::{ants::Ant, connect};
use rand::seq::SliceRandom;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use twitter_v2::authorization::Oauth1aToken;
use twitter_v2::TwitterApi;

async fn post_tweet(ant_content: String) -> Option<twitter_v2::Tweet> {
    info!("Tweeting with ant: {}", ant_content);

    let consumer_key = dotenv::var("TWITTER_API_CONSUMER_KEY").unwrap();
    let consumer_secret = dotenv::var("TWITTER_API_CONSUMER_SECRET").unwrap();
    let access_token = dotenv::var("TWITTER_API_ACCESS_TOKEN").unwrap();
    let access_token_secret = dotenv::var("TWITTER_API_ACCESS_TOKEN_SECRET").unwrap();

    let token = Oauth1aToken::new(
        consumer_key,
        consumer_secret,
        access_token,
        access_token_secret,
    );

    let client = TwitterApi::new(token);
    client
        .post_tweet()
        .text(ant_content)
        .send()
        .await
        .unwrap_or_else(|e| panic!("Error sending tweet: {e}"))
        .into_data()
}

async fn cron_tweet() {
    info!("Starting cron...");
    let dao = connect().await;

    info!("Getting random ant choice...");
    let random_ant: Ant = {
        let read_ants = dao.ants.read().await;
        let ants = read_ants
            .get_all()
            .await
            .iter()
            .filter(|&ant| ant.tweeted == Tweeted::NotTweeted)
            .map(|&x| x.clone())
            .collect::<Vec<Ant>>();
        ants.choose(&mut rand::thread_rng())
            .unwrap_or_else(|| panic!("Failed to get a random choice!"))
            .clone().clone()
    };

    info!("Tweeting...");
    let res = post_tweet(random_ant.ant_name).await;
    assert!(res.is_some(), "Failed to tweet!");

    info!("Saving result to DB...");
    dao.ants
        .write()
        .await
        .add_ant_tweet(&random_ant.ant_id)
        .await;
    info!("Cron tasks done, exiting...");
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_file(true)
        .with_ansi(false)
        .with_writer(tracing_appender::rolling::hourly(
            "./logs/ant-on-twitter",
            "ant-on-twitter.log",
        ))
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    dotenv::dotenv().unwrap_or_else(|e| panic!("Environment error: {e}"));
    let mut scheduler = JobScheduler::new().await.unwrap();

    scheduler
        .add(
            // 6pm MST is midnight UTC
            Job::new_async("0 0 0 * * *", |_, __| {
                Box::pin(async move {
                    cron_tweet().await;
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();

    // Start the scheduler
    scheduler.start().await.unwrap();

    loop {
        let sleep_time: Duration = scheduler
            .time_till_next_job()
            .await
            .unwrap_or(Some(Duration::from_secs(10)))
            .unwrap_or(Duration::from_secs(10));
        info!("Sleeping for {} seconds!", sleep_time.as_secs());
        tokio::time::sleep(sleep_time).await;
    }
}
