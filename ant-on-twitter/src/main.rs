use twitter_v2::authorization::Oauth1aToken;
use twitter_v2::TwitterApi;

async fn post_tweet(client: TwitterApi<Oauth1aToken>, ant_content: String) -> () {
    client
        .post_tweet()
        .text(ant_content)
        .send()
        .await
        .unwrap_or_else(|e| panic!("Error sending tweet: {}", e))
        .into_data()
        .expect("The tweet was tweeted");
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap_or_else(|e| panic!("Environment error: {}", e));
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

    let should_tweet = false;
    if should_tweet {
        post_tweet(client, "ant who's grinding ($)".to_owned()).await;
    }

    // let auth: Oauth2Token = serde_json::from_str(&stored_oauth2_token)?;
    // let my_followers = TwitterApi::new(auth)
    //     .with_user_ctx()
    //     .await?
    //     .get_my_followers()
    //     .user_fields([UserField::Username])
    //     .max_results(20)
    //     .send()
    //     .await?
    //     .into_data();
}
