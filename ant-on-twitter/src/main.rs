use std::time::Duration;

use ant_data_farm::ants::Tweeted;
use ant_data_farm::{ants::Ant, connect};
use rand::seq::SliceRandom;
use tokio_cron_scheduler::{Job, JobScheduler};
use twitter_v2::authorization::Oauth1aToken;
use twitter_v2::TwitterApi;

async fn post_tweet(
    client: TwitterApi<Oauth1aToken>,
    ant_content: String,
) -> Option<twitter_v2::Tweet> {
    client
        .post_tweet()
        .text(ant_content)
        .send()
        .await
        .unwrap_or_else(|e| panic!("Error sending tweet: {}", e))
        .into_data()
}

async fn cron_tweet() {
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

    let dao = connect().await;

    let random_ant: Ant = {
        let read_ants = dao.ants.read().await;
        let ants = read_ants
            .get_all_ants()
            .iter()
            .filter(|&ant| ant.tweeted == Tweeted::NotTweeted)
            .map(|&x| x.clone())
            .collect::<Vec<Ant>>();
        ants.choose(&mut rand::thread_rng())
            .unwrap_or_else(|| panic!("Failed to get a random choice!"))
            .clone()
            .to_owned()
    };

    let client = TwitterApi::new(token);

    println!("Tweeting with ant: {:?}", random_ant);
    let res = post_tweet(client, random_ant.ant_name).await;
    if res.is_none() {
        panic!("Failed to tweet!");
    }

    dao.ants
        .write()
        .await
        .add_ant_tweet(random_ant.ant_id)
        .await;
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap_or_else(|e| panic!("Environment error: {}", e));
    let mut scheduler = JobScheduler::new().await.unwrap();

    scheduler
        .add(
            Job::new_async("0 0 18 * * *", |_, __| {
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
        tokio::time::sleep(sleep_time).await;
    }
}
