mod util;

use ant_data_farm::{ants::Tweeted, AntDataFarmClient, DaoTrait, DatabaseConfig};
use chrono::Duration;

use tracing::debug;
use util::{logging, test_fixture};

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn more_than_500_ants() {
    let fixture = test_fixture();
    let container = fixture.docker.run(fixture.image);
    let port = container.get_host_port_ipv4(5432);
    let dao = AntDataFarmClient::new(Some(DatabaseConfig {
        port: Some(port),
        creds: None,
        host: None,
    }))
    .await
    .expect("Connected!");

    let ants = dao.ants.read().await;
    let all_ants = ants.get_all_released().await;
    assert!(all_ants.len() >= 500);
}

#[rstest::rstest(logging as _logging)]
#[tokio::test(flavor = "multi_thread")]
async fn add_tweeted(_logging: &()) {
    let fixture = test_fixture();
    let container = fixture.docker.run(fixture.image);
    debug!("Ran fixture!");
    let port = container.get_host_port_ipv4(5432);
    let dao = AntDataFarmClient::new(Some(DatabaseConfig {
        port: Some(port),
        creds: None,
        host: None,
    }))
    .await
    .expect("Connected!");

    let ant_id = {
        let ants = dao.ants.read().await;
        ants.get_all().await.last().unwrap().ant_id
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
    let found_ant = ants.get_one_by_id(&ant_id).await;
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
