use ant_data_farm::{
    ants::Ant, ants::Tweeted, tweets::ScheduledTweet, AntDataFarmClient, DatabaseConfig,
    DatabaseCredentials,
};
use ant_library::set_global_logs;
use chrono::{DateTime, Timelike, Utc};
use rand::seq::IteratorRandom;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;
use twitter_v2::authorization::Oauth1aToken;
use twitter_v2::TwitterApi;

#[derive(Debug, Clone)]
struct TwitterCredentials {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}

async fn post_tweet(tweet_content: String, creds: TwitterCredentials) -> Option<twitter_v2::Tweet> {
    info!("Tweeting with content: {}", tweet_content);

    let token = Oauth1aToken::new(
        creds.consumer_key,
        creds.consumer_secret,
        creds.access_token,
        creds.access_token_secret,
    );

    let client = TwitterApi::new(token);
    client
        .post_tweet()
        .text(tweet_content)
        .send()
        .await
        .unwrap_or_else(|e| panic!("Error sending tweet: {e}"))
        .into_data()
}

async fn ant_client(config: DatabaseConfig) -> AntDataFarmClient {
    match AntDataFarmClient::new(Some(config)).await {
        Err(e) => {
            panic!("Failed to initialize database: {}", e);
        }
        Ok(client) => client,
    }
}

/// Check if there is a scheduled ant within 24 hours that needs to be tweeted instead. If so, tweet that.
async fn choose_scheduled_ants(
    client: &AntDataFarmClient,
) -> Result<Option<ScheduledTweet>, anyhow::Error> {
    let read_tweets = client.tweets.read().await;
    match read_tweets.get_next_scheduled_tweet().await? {
        None => {
            return Ok(None);
        }
        Some(tweet) => {
            let till_scheduled: i64 =
                DateTime::signed_duration_since(tweet.scheduled_at, Utc::now()).num_hours();
            info!("Next scheduled tweet is {} hours away...", till_scheduled);

            // The schedule can be placed anytime that day, could be in the past a bit.
            if till_scheduled < 24 && till_scheduled > -24 {
                return Ok(Some(tweet));
            } else {
                return Ok(None);
            }
        }
    }
}

/// From the entire list of released ants, choose one randomly
async fn choose_random_ant(client: &AntDataFarmClient) -> Result<Ant, anyhow::Error> {
    let read_ants = client.ants.read().await;
    let ants = read_ants
        .get_all_released()
        .await?
        .into_iter()
        .filter(|ant| ant.tweeted == Tweeted::NotTweeted)
        .collect::<Vec<Ant>>();
    let ant = ants
        .into_iter()
        .choose(&mut rand::rng())
        .unwrap_or_else(|| panic!("Failed to get a random choice!"))
        .clone();

    return Ok(ant);
}

async fn cron_tweet() -> Result<(), anyhow::Error> {
    info!("Starting cron_tweet()...");
    let config: Config = get_config().expect("Getting config failed!");

    info!("Beginning database connection...");
    let client = ant_client(config.database).await;

    info!("Getting random ant choice...");
    if let Some(tweet) = choose_scheduled_ants(&client).await? {
        info!("Tweeting scheduled tweet...");

        let mut tweet_content: String = tweet.tweet_prefix.unwrap_or("".to_string());
        for ant in tweet.ants_to_tweet.iter() {
            tweet_content.push_str(ant.ant_content.as_str());
            tweet_content.push('\n');
        }
        tweet_content.push_str(tweet.tweet_suffix.unwrap_or("".to_string()).as_str());

        let res = post_tweet(tweet_content, config.twitter).await;
        assert!(res.is_some(), "Failed to tweet!");

        for ant in tweet.ants_to_tweet.iter() {
            info!("Saving ant content '{}' as tweeted...", ant.ant_content);
            client.ants.write().await.add_ant_tweet(&ant.ant_id).await?;
        }

        info!("Cron tasks done, exiting...");
        return Ok(());
    } else {
        let random_ant = choose_random_ant(&client).await?;

        info!("Tweeting...");
        let res = post_tweet(random_ant.ant_name, config.twitter).await;
        assert!(res.is_some(), "Failed to tweet!");

        info!("Saving result to DB...");
        client
            .ants
            .write()
            .await
            .add_ant_tweet(&random_ant.ant_id)
            .await?;
        info!("Cron tasks done, exiting...");
        return Ok(());
    }
}

#[derive(Debug, Clone)]
struct Config {
    pub twitter: TwitterCredentials,
    pub database: DatabaseConfig,
}

fn get_config() -> Result<Config, dotenv::Error> {
    info!("Loading creds from env...");
    dotenv::dotenv()?;

    let config = Config {
        twitter: TwitterCredentials {
            consumer_key: dotenv::var("TWITTER_API_CONSUMER_KEY")?,
            consumer_secret: dotenv::var("TWITTER_API_CONSUMER_SECRET")?,
            access_token: dotenv::var("TWITTER_API_ACCESS_TOKEN")?,
            access_token_secret: dotenv::var("TWITTER_API_ACCESS_TOKEN_SECRET")?,
        },
        database: DatabaseConfig {
            creds: Some(DatabaseCredentials {
                database_name: dotenv::var("DB_PG_NAME")?,
                database_user: dotenv::var("DB_PG_USER")?,
                database_password: dotenv::var("DB_PG_PASSWORD")?,
            }),
            host: Some(dotenv::var("DB_HOST")?),
            port: Some(7000),
            migration_dir: None,
        },
    };

    info!("Config constructed successfully.");
    return Ok(config);
}

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

    // Check the connection to the database by creating a dummy "test" client at the front.
    ant_client(
        get_config()
            .expect("Getting environment variables failed!")
            .database,
    )
    .await;

    scheduler
        .add(
            // 6pm MST is midnight UTC
            Job::new_async(
                format!("0 0 {} * * *", utc_hour_to_tweet).as_str(),
                |_, __| {
                    Box::pin(async move {
                        info!("Entering cron_tweet()...");
                        cron_tweet().await.expect("Cron tweet failed!");
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
