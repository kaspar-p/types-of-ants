use ant_who_tweets::get_config;
use tracing::info;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn can_comment_a_tweet() {
    // let (pg, db_client) = test_database_client().await;

    let twitter_conf = get_config().unwrap().twitter;
    assert!(twitter_conf.handle.contains("beta"));
    info!("{:?}", twitter_conf);

    // let tweets = cron_tweet(Config {
    //     twitter: twitter_conf,
    //     database: DatabaseConfig {
    //         port: Some(pg.settings().port),
    //         creds: Some(DatabaseCredentials {
    //             database_name: "test".to_string(),
    //             database_user: pg.settings().username.clone(),
    //             database_password: pg.settings().password.clone(),
    //         }),
    //         host: Some(pg.settings().host.clone()),
    //         migration_dir: Some(
    //             PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    //                 .join("..")
    //                 .join("ant-data-farm/data/sql"),
    //         ),
    //     },
    // })
    // .await
    // .unwrap();

    assert!(1 == 2);
}
