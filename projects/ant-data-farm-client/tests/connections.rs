mod fixture;

use ant_data_farm::{AntDataFarmClient, DaoTrait, DatabaseConfig, DatabaseCredentials};

use fixture::test_fixture;
use testcontainers::runners::AsyncRunner;

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn connection_resetting_works_fine() {
    let dao = {
        let fixture = test_fixture("connection_resetting_works_fine_1", Some(13654)).await;
        let container = fixture.image.start().await.unwrap();

        let host = container.get_host().await.unwrap().to_string();
        assert_eq!(host, "localhost");

        let port = container.get_host_port_ipv4(5432).await.unwrap();
        assert_eq!(port, 13654);

        let dao = AntDataFarmClient::new(Some(DatabaseConfig {
            port: Some(port),
            creds: Some(DatabaseCredentials {
                database_name: "typesofants".to_string(),
                database_user: "test".to_string(),
                database_password: "test".to_string(),
            }),
            host: Some(host),
            migration_dir: None,
        }))
        .await
        .expect("Connected!");

        let ants = dao.ants.read().await.get_all().await.unwrap();
        assert!(ants.len() > 0);

        // Moving this out of the scope kills the underlying container!
        dao
    };

    {
        // Query again to make sure that new container works
        let fixture = test_fixture("connection_resetting_works_fine_2", Some(13654)).await;
        let container = fixture.image.start().await.unwrap();

        // The connection parameters have to still be the same!
        let host = container.get_host().await.unwrap().to_string();
        assert_eq!(host, "localhost");

        let port = container.get_host_port_ipv4(5432).await.unwrap();
        assert_eq!(port, 13654);

        let ants = dao.ants.read().await.get_all().await.unwrap();
        assert!(ants.len() > 0);
    }
}
