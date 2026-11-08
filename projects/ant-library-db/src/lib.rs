use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// The database port.
    pub port: u16,
    /// The credentials to connect to the database.
    pub database_name: String,
    pub database_user: String,
    pub database_password: String,
    /// The host of the database.
    pub host: String,
    /// On client startup, execute the SQL within the directory to bootstrap schemas and databases.
    /// Executes in the order of the vector.
    pub migration_dirs: Vec<PathBuf>,
}
