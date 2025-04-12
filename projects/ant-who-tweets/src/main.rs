use ant_data_farm::{
    ants::Ant, ants::Tweeted, AntDataFarmClient, DatabaseConfig, DatabaseCredentials,
};
use chrono::Timelike;
use rand::seq::SliceRandom;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use twitter_v2::authorization::Oauth1aToken;
use twitter_v2::TwitterApi;

#[derive(Debug, Clone)]
struct TwitterCredentials {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}

async fn post_tweet(ant_content: String, creds: TwitterCredentials) -> Option<twitter_v2::Tweet> {
    info!("Tweeting with ant: {}", ant_content);

    let token = Oauth1aToken::new(
        creds.consumer_key,
        creds.consumer_secret,
        creds.access_token,
        creds.access_token_secret,
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

async fn ant_client(config: DatabaseConfig) -> AntDataFarmClient {
    match AntDataFarmClient::new(Some(config)).await {
        Err(e) => {
            panic!("Failed to initialize database: {}", e);
        }
        Ok(client) => client,
    }
}

async fn cron_tweet() -> () {
    info!("Starting cron_tweet()...");
    let config: Config = get_config().expect("Getting config failed!");

    info!("Beginning database connection...");
    let client = ant_client(config.database).await;

    info!("Getting random ant choice...");
    let random_ant: Ant = {
        let read_ants = client.ants.read().await;
        let ants = read_ants
            .get_all_released()
            .await
            .iter()
            .filter(|&ant| ant.tweeted == Tweeted::NotTweeted)
            .map(|&x| x.clone())
            .collect::<Vec<Ant>>();
        ants.choose(&mut rand::thread_rng())
            .unwrap_or_else(|| panic!("Failed to get a random choice!"))
            .clone()
    };

    info!("Tweeting...");
    let res = post_tweet(random_ant.ant_name, config.twitter).await;
    assert!(res.is_some(), "Failed to tweet!");

    info!("Saving result to DB...");
    client
        .ants
        .write()
        .await
        .add_ant_tweet(&random_ant.ant_id)
        .await;
    info!("Cron tasks done, exiting...");
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
        },
    };

    info!("Config constructed successfully.");
    return Ok(config);
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
    let scheduler = JobScheduler::new().await.unwrap();

    // 8pm EST is 6pm MST
    let hour_to_tweet = 0; // spring/summer

    // let hour_to_tweet = 0; // fall/winter
    let local = chrono::offset::Local::now().hour();

    info!(
        "Starting up! Local hour: {}, hour to tweet: {}",
        local, hour_to_tweet
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
            Job::new_async(format!("0 0 {} * * *", hour_to_tweet).as_str(), |_, __| {
                Box::pin(async move {
                    info!("Entering cron_tweet()...");
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
        info!("Sleeping 1800 seconds...");
        tokio::time::sleep(Duration::from_secs(1800)).await;
    }
}
