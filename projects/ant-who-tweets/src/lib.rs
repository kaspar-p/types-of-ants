use ant_data_farm::{ants::Ant, ants::Tweeted, tweets::ScheduledTweet, AntDataFarmClient};
use ant_library::db::{DatabaseConfig, TypesOfAntsDatabase};
use chrono::{DateTime, Utc};
use rand::seq::IteratorRandom;
use tracing::info;
use twitter_v2::authorization::Oauth1aToken;
use twitter_v2::TwitterApi;

#[derive(Debug, Clone)]
pub struct TwitterCredentials {
    pub handle: String,
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}

async fn post_tweet(
    creds: TwitterCredentials,
    tweet_content: String,
    comments: Vec<String>,
) -> Vec<twitter_v2::Tweet> {
    info!(
        "Tweeting from @{}, content: {} and comments {}",
        creds.handle,
        tweet_content,
        comments.join("\n")
    );

    let token = Oauth1aToken::new(
        creds.consumer_key,
        creds.consumer_secret,
        creds.access_token,
        creds.access_token_secret,
    );

    let client = TwitterApi::new(token);

    let mut tweets: Vec<twitter_v2::Tweet> = vec![];
    let tweet = client
        .post_tweet()
        .text(tweet_content)
        .send()
        .await
        .expect("posting tweet")
        .into_data()
        .expect("no tweet payload");

    info!("tweeted: {:?}", tweet);
    let orig_tweet = tweet.clone();
    tweets.push(tweet);

    for comment in comments {
        let comment_tweet = client
            .post_tweet()
            .text(comment)
            .in_reply_to_tweet_id(orig_tweet.id)
            .send()
            .await
            .expect("posting comment")
            .into_data()
            .expect("no comment payload");

        info!("commented: {:?}", comment_tweet);
        tweets.push(comment_tweet);
    }

    return tweets;
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

pub async fn cron_tweet(config: Config) -> Result<Vec<twitter_v2::Tweet>, anyhow::Error> {
    info!("Starting cron_tweet()...");

    info!("Beginning database connection...");
    let client = AntDataFarmClient::connect(&config.database).await?;

    info!("Getting random ant choice...");
    if let Some(tweet) = choose_scheduled_ants(&client).await? {
        info!("Tweeting scheduled tweet...");

        let mut tweet_content: String = "".to_owned();
        tweet_content.push_str(tweet.tweet_prefix.unwrap_or("".to_string()).as_str());
        for ant in tweet.ants_to_tweet.iter() {
            tweet_content.push_str(ant.ant_content.as_str());
            tweet_content.push('\n');
        }
        tweet_content.push_str(tweet.tweet_suffix.unwrap_or("".to_string()).as_str());

        let tweets = post_tweet(
            config.twitter,
            tweet_content,
            vec![format!(
                "tweet scheduled by: {}",
                tweet.scheduled_by_user_name
            )],
        )
        .await;

        for ant in tweet.ants_to_tweet.iter() {
            info!("Saving ant content '{}' as tweeted...", ant.ant_content);
            client.ants.write().await.add_ant_tweet(&ant.ant_id).await?;
        }

        info!("Marking {} as tweeted...", tweet.scheduled_tweet_id);
        client
            .tweets
            .write()
            .await
            .mark_scheduled_tweet_tweeted(tweet.scheduled_tweet_id)
            .await?;

        info!("Cron tasks done, exiting...");
        return Ok(tweets);
    } else {
        let random_ant = choose_random_ant(&client).await?;

        info!("Tweeting...");
        let tweets = post_tweet(config.twitter, random_ant.ant_name, vec![]).await;

        info!("Saving result to DB...");
        client
            .ants
            .write()
            .await
            .add_ant_tweet(&random_ant.ant_id)
            .await?;
        info!("Cron tasks done, exiting...");
        return Ok(tweets);
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub twitter: TwitterCredentials,
    pub database: DatabaseConfig,
}

pub fn get_config() -> Result<Config, anyhow::Error> {
    info!("Loading creds from env...");
    dotenv::dotenv()?;

    let config = Config {
        twitter: TwitterCredentials {
            handle: dotenv::var("TWITTER_API_ACCOUNT_HANDLE")?,
            consumer_key: ant_library::secret::load_secret("twitter_consumer_key")?,
            consumer_secret: ant_library::secret::load_secret("twitter_consumer_secret")?,
            access_token: ant_library::secret::load_secret("twitter_access_token")?,
            access_token_secret: ant_library::secret::load_secret("twitter_access_token_secret")?,
        },
        database: DatabaseConfig {
            database_name: ant_library::secret::load_secret("ant_data_farm_db")?,
            database_user: ant_library::secret::load_secret("ant_data_farm_user")?,
            database_password: ant_library::secret::load_secret("ant_data_farm_password")?,
            host: dotenv::var("ANT_DATA_FARM_HOST")?,
            port: dotenv::var("ANT_DATA_FARM_PORT")?.parse::<u16>()?,
            migration_dirs: vec![],
        },
    };

    info!("Config constructed successfully.");
    return Ok(config);
}
