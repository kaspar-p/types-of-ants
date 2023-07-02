mod util;

use ant_data_farm::{ants::Tweeted, connect_port, Dao, DaoTrait};
use chrono::Duration;
use testcontainers::{clients::Cli, images::generic::GenericImage, Container};
use util::{logging, test_fixture, TestFixture};

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn more_than_500_ants() {
    let fixture = test_fixture().await;
    let container = fixture.docker.run(fixture.image);
    let port = container.get_host_port_ipv4(5432);
    let dao = connect_port(port).await;

    let ants = dao.ants.read().await;
    let all_ants = ants.get_all().await;
    assert!(all_ants.len() >= 500);
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn add_tweeted(logging: &()) {
    let fixture = test_fixture().await;
    let container = fixture.docker.run(fixture.image);
    let port = container.get_host_port_ipv4(5432);
    let dao = connect_port(port).await;

    let ant_id = {
        let ants = dao.ants.read().await;
        ants.get_all().await.last().unwrap().ant_id
    };

    let mut ants = dao.ants.write().await;
    let ant = ants.add_ant_tweet(&ant_id).await.unwrap();
    // let ant = ants.get_one_by_id(&ant_id).await.unwrap();
    println!("{:#?}", ant);
    match ant.tweeted {
        Tweeted::NotTweeted => panic!("Ant should have tweeted!"),
        Tweeted::Tweeted(time) => assert!(time
            .signed_duration_since(chrono::offset::Utc::now())
            .le(&Duration::seconds(10))),
    }
}
