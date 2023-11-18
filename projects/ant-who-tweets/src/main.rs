use ant_data_farm::ants::Ant;
use ant_data_farm::ants::Tweeted;
use ant_data_farm::{DatabaseConfig, DatabaseCredentials};
use chrono::Timelike;
use rand::seq::SliceRandom;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::error;
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

async fn cron_tweet() -> () {
    info!("Starting cron_tweet()...");
    let config: Config = get_config().expect("Getting config failed!");

    info!("Beginning database connection...");
    let dao = match ant_data_farm::connect_config(config.database).await {
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            error!("Ending CRON early!");
            return;
        }
        Ok(dao) => dao,
    };

    info!("Getting random ant choice...");
    let random_ant: Ant = {
        let read_ants = dao.ants.read().await;
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
    dao.ants
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
    let twitter_env_key = "TWITTER_CREDS_FILE";
    let db_env_key = "DATABASE_CREDS_FILE";

    let twitter_env =
        dotenv::var(twitter_env_key).expect("Twitter file credential not in environment!");
    let db_env = dotenv::var(db_env_key).expect("Database file credentials not in environment!");
    dotenv::from_path(twitter_env).expect("Twitter creds file not found!");
    dotenv::from_path(db_env).expect("Database creds file not found!");

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
    println!("Config: {:#?}", config);
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

    let local = chrono::offset::Local::now().hour();
    let utc = chrono::offset::Utc::now().hour();
    let real_diff: u32 = {
        if utc > local {
            utc - local
        } else {
            (24 + utc) - local
        }
    };
    let expected_diff = 6;
    let hour_offset = expected_diff - real_diff;

    info!(
        "Starting up! Local hours: {}, UTC hours: {}, hour to tweet: {}",
        local, utc, hour_offset
    );

    scheduler
        .add(
            // 6pm MST is midnight UTC
            Job::new_async(format!("0 0 {} * * *", hour_offset).as_str(), |_, __| {
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
