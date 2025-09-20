mod fixture;

use ant_data_farm::{
    ants::Tweeted, AntDataFarmClient, DaoTrait, DatabaseConfig, DatabaseCredentials,
};
use chrono::Duration;

use fixture::{logging, test_fixture};
use testcontainers::runners::AsyncRunner;
use tracing::debug;

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn more_than_500_ants() {
    let fixture = test_fixture("more_than_500_ants", None).await;
    let container = fixture.image.start().await.unwrap();

    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let dao = AntDataFarmClient::new(Some(DatabaseConfig {
        port: Some(port),
        creds: Some(DatabaseCredentials {
            database_name: "typesofants".to_string(),
            database_user: "test".to_string(),
            database_password: "test".to_string(),
        }),
        host: Some("localhost".to_string()),
        migration_dir: None,
    }))
    .await
    .expect("Connected!");

    let ants = dao.ants.read().await;
    let all_ants = ants.get_all_released().await.unwrap();
    assert!(all_ants.len() >= 500);
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn user_gets_created() {
    let fixture = test_fixture("user_gets_created", None).await;
    let container = fixture.image.start().await.unwrap();

    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let dao = AntDataFarmClient::new(Some(DatabaseConfig {
        port: Some(port),
        creds: Some(DatabaseCredentials {
            database_name: "typesofants".to_string(),
            database_user: "test".to_string(),
            database_password: "test".to_string(),
        }),
        host: Some("localhost".to_string()),
        migration_dir: None,
    }))
    .await
    .expect("Connected!");

    let mut users = dao.users.write().await;

    users
        .create_user(
            "integ-user".to_string(),
            "integ-user-password".to_string(),
            "user".to_string(),
        )
        .await
        .unwrap();

    let user_by_name = users
        .get_one_by_user_name("integ-user")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user_by_name.username, "integ-user");
}

#[rstest::rstest(logging as _logging)]
#[tokio::test(flavor = "multi_thread")]
async fn see_scheduled_tweets(_logging: &()) {
    let fixture = test_fixture("see_scheduled_tweets", None).await;
    let container = fixture.image.start().await.unwrap();

    let port = container.get_host_port_ipv4(5432).await.unwrap();
    debug!("Ran fixture!");
    let dao = AntDataFarmClient::new(Some(DatabaseConfig {
        port: Some(port),
        creds: Some(DatabaseCredentials {
            database_name: "typesofants".to_string(),
            database_user: "test".to_string(),
            database_password: "test".to_string(),
        }),
        host: Some("localhost".to_string()),
        migration_dir: None,
    }))
    .await
    .expect("Connected!");

    let scheduled = dao
        .tweets
        .read()
        .await
        .get_next_scheduled_tweet()
        .await
        .unwrap();

    assert!(scheduled.is_some() || scheduled.is_none());
}

#[rstest::rstest(logging as _logging)]
#[tokio::test(flavor = "multi_thread")]
async fn add_tweeted(_logging: &()) {
    let fixture = test_fixture("add_tweeted", None).await;
    let container = fixture.image.start().await.unwrap();

    let port = container.get_host_port_ipv4(5432).await.unwrap();
    debug!("Ran fixture!");
    let dao = AntDataFarmClient::new(Some(DatabaseConfig {
        port: Some(port),
        creds: Some(DatabaseCredentials {
            database_name: "typesofants".to_string(),
            database_user: "test".to_string(),
            database_password: "test".to_string(),
        }),
        host: Some("localhost".to_string()),
        migration_dir: None,
    }))
    .await
    .expect("Connected!");

    let ant_id = {
        let ants = dao.ants.read().await;
        ants.get_all().await.unwrap().last().unwrap().ant_id
    };

    {
        let mut write_ants = dao.ants.write().await;
        let ant = write_ants.add_ant_tweet(&ant_id).await.unwrap();
        println!("{ant:#?}");
        match &ant.tweeted {
            Tweeted::NotTweeted => panic!("Ant should have tweeted!"),
            Tweeted::Tweeted(time) => assert!(time
                .signed_duration_since(chrono::offset::Utc::now())
                .le(&Duration::seconds(10))),
        }
    }

    let ants = dao.ants.read().await;
    let found_ant = ants.get_one_by_id(&ant_id).await.unwrap();
    match found_ant {
        None => panic!("Failed to get ant again!"),
        Some(found_ant) => match &found_ant.tweeted {
            Tweeted::NotTweeted => panic!("Ant should have tweeted!"),
            Tweeted::Tweeted(time) => assert!(time
                .signed_duration_since(chrono::offset::Utc::now())
                .le(&Duration::seconds(10))),
        },
    }
}
