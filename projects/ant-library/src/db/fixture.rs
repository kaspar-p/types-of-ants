use std::path::PathBuf;

use postgresql_embedded::PostgreSQL;

use crate::db::DatabaseConfig;

pub async fn test_database_config(project: &str) -> (PostgreSQL, DatabaseConfig) {
    let mut pg = PostgreSQL::new(postgresql_embedded::Settings {
        temporary: true,
        ..Default::default()
    });
    pg.setup().await.unwrap();
    pg.start().await.unwrap();

    pg.create_database("typesofants").await.unwrap();

    let config = DatabaseConfig {
        port: pg.settings().port.clone(),
        host: pg.settings().host.clone(),
        database_name: "typesofants".to_string(),
        database_password: pg.settings().password.clone(),
        database_user: pg.settings().username.clone(),
        migration_dir: Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(project)
                .join("migrations"),
        ),
    };

    return (pg, config);
}
