use chrono::{DateTime, Utc};
use tokio_postgres::{Error, NoTls};

use crate::tests::ping::StatusData;

pub struct Database {
    client: tokio_postgres::Client,
}

impl Database {
    pub async fn insert_status_data(&self, data: Vec<StatusData>) -> Result<(), Error> {
        println!("Inserting data...");
        let values = data
            .iter()
            .map(|status| status.to_test_sql_row(1))
            .collect::<Vec<String>>();
        let query = "insert into test_instance (test_instance_test_id, test_instance_start_time, test_instance_end_time, test_instance_status) values".to_owned() + &values.join(",");
        self.client.query(query.as_str(), &[]).await?;

        Ok(())
    }
}

pub async fn connect() -> Result<Database, Error> {
    let db_name = "typesofants";
    let user = "typesofants";
    let pw = ""; // Add password here!

    let connection_string = format!("postgresql://{user}:{pw}@localhost:7000/{db_name}");
    let (client, connection) = tokio_postgres::connect(connection_string.as_str(), NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });

    println!("Fetching data...");
    let rows = client.query("select * from registered_user;", &[]).await?;

    for row in rows {
        let user_id: i32 = row.get("user_id");
        let user_name: String = row.get("user_name");
        let user_joined: DateTime<Utc> = row.get("user_joined");
        println!(
            "user_id: {}, user_name: {}, user_joined: {}",
            user_id, user_name, user_joined
        );
    }

    Ok(Database { client })
}
