use std::path::PathBuf;

use ant_data_farm::{AntDataFarmClient, DatabaseConfig, DatabaseCredentials};
use ant_who_tweets::{cron_tweet, get_config, Config};
use postgresql_embedded::PostgreSQL;
use tracing::info;
use tracing_test::traced_test;

async fn test_database_client() -> (PostgreSQL, AntDataFarmClient) {
    let mut pg = PostgreSQL::new(postgresql_embedded::Settings {
        temporary: true,
        ..Default::default()
    });
    pg.setup().await.unwrap();
    pg.start().await.unwrap();

    pg.create_database("test").await.unwrap();

    let client = AntDataFarmClient::new(Some(DatabaseConfig {
        port: Some(pg.settings().port),
        host: Some(pg.settings().host.clone()),
        creds: Some(DatabaseCredentials {
            database_name: "test".to_string(),
            database_password: pg.settings().password.clone(),
            database_user: pg.settings().username.clone(),
        }),
        migration_dir: Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("ant-data-farm/data/sql"),
        ),
    }))
    .await
    .expect("connection failed");

    return (pg, client);
}

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
