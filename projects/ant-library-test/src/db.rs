use std::{fs::exists, path::PathBuf};

use postgresql_embedded::PostgreSQL;

use ant_library::db::DatabaseConfig;

pub struct TestDatabase {
    _db: PostgreSQL,
    pub config: DatabaseConfig,
}

impl TestDatabase {
    pub async fn new(project: &str) -> Self {
        TestDatabase::with_settings(
            project,
            postgresql_embedded::Settings {
                ..Default::default()
            },
        )
        .await
    }

    pub async fn with_settings(project: &str, mut settings: postgresql_embedded::Settings) -> Self {
        let root = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));

        let pg_tmp_dir = root.join("build").join("pgtmp");
        std::fs::create_dir_all(&pg_tmp_dir).unwrap();

        settings.temporary = true;
        settings.installation_dir = pg_tmp_dir;
        let mut pg = PostgreSQL::new(settings);

        pg.setup().await.unwrap();
        pg.start().await.unwrap();

        pg.create_database("typesofants").await.unwrap();

        let mut migration_dirs = vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(project)
                .join("migrations"),
        ];
        let seed_data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(project)
            .join("seed-data");
        if exists(&seed_data_path).unwrap() {
            migration_dirs.push(seed_data_path);
        }

        let config = DatabaseConfig {
            port: pg.settings().port.clone(),
            host: pg.settings().host.clone(),
            database_name: "typesofants".to_string(),
            database_password: pg.settings().password.clone(),
            database_user: pg.settings().username.clone(),
            migration_dirs,
        };

        return Self { _db: pg, config };
    }
}

pub async fn test_database_config(project: &str) -> (PostgreSQL, DatabaseConfig) {
    let root = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));

    let pg_tmp_dir = root.join("build").join("pgtmp");
    std::fs::create_dir_all(&pg_tmp_dir).unwrap();

    let mut pg = PostgreSQL::new(postgresql_embedded::Settings {
        temporary: true,
        installation_dir: pg_tmp_dir,
        ..Default::default()
    });
    pg.setup().await.unwrap();
    pg.start().await.unwrap();

    pg.create_database("typesofants").await.unwrap();

    let mut migration_dirs = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(project)
            .join("migrations"),
    ];
    let seed_data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(project)
        .join("seed-data");
    if exists(&seed_data_path).unwrap() {
        migration_dirs.push(seed_data_path);
    }

    let config = DatabaseConfig {
        port: pg.settings().port.clone(),
        host: pg.settings().host.clone(),
        database_name: "typesofants".to_string(),
        database_password: pg.settings().password.clone(),
        database_user: pg.settings().username.clone(),
        migration_dirs,
    };

    return (pg, config);
}
