use chrono;

use crate::db::Database;

const URLS: &'static [&'static str] = &[
    "http://typesofants.org",
    "http://www.typesofants.org",
    "http://6krill.com",
];

#[derive(Debug)]
struct StatusData<'a> {
    url: &'a str,
    timestamp: chrono::DateTime<chrono::offset::Utc>,
    healthy: bool,
    status: reqwest::StatusCode,
}

impl std::fmt::Display for StatusData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "[{}] [{}]: {{ healthy: {}, status: {} }}",
            self.timestamp.format("%d%b%Y %T"),
            self.url,
            self.healthy,
            self.status
        )?;
        Ok(())
    }
}

/**
 * From a web response, construct data to go into a database
 */
fn construct_data(url: &str, res: reqwest::blocking::Response) -> StatusData {
    StatusData {
        url,
        timestamp: std::time::SystemTime::now().into(),
        healthy: res.status().is_success(),
        status: res.status(),
    }
}

/**
 * Ping the relevant URLs to see if they are up
 *
 * TODO: make the request non-blocking
 */
pub fn ping_test(database: &Database) -> impl Fn() -> Result<(), reqwest::Error> {
    return &|| {
        let client = reqwest::blocking::Client::new();
        for url in URLS {
            let response = client.get(url.to_string()).send()?;
            let data = construct_data(url, response);
            println!("{}", data);
            database.connection.execute("INSERT INTO tests (")
        }

        return Ok(());
    };
}
