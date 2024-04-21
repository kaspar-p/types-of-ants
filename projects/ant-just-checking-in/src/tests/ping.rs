use chrono::{DateTime, Utc};
use reqwest::StatusCode;

const URLS: &[&str] = &[
    // Main
    "http://typesofants.org",
    "https://typesofants.org",
    "http://www.typesofants.org",
    "https://www.typesofants.org",
    // Aliases
    "http://typeofants.org",
    "https://typeofants.org",
    "http://typesofant.org",
    "https://typesofant.org",
    "http://typeofant.org",
    "https://typeofant.org",
    // Beta endpoints
    "http://beta.typesofants.org",
    "https://beta.typesofants.org",
    // Other endpoints
    "http://6krill.com",
    "https://6krill.com",
];

#[derive(Debug)]
pub struct StatusData {
    url: String,
    start_timestamp: DateTime<Utc>,
    end_timestamp: DateTime<Utc>,
    healthy: bool,
    status: reqwest::StatusCode,
}

impl StatusData {
    pub fn new(
        url: String,
        start_timestamp: DateTime<Utc>,
        healthy: bool,
        status: reqwest::StatusCode,
    ) -> Self {
        StatusData {
            url,
            start_timestamp,
            end_timestamp: start_timestamp,
            healthy,
            status,
        }
    }
}

impl std::fmt::Display for StatusData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "[{}] [{}]: {{ healthy: {}, status: {} }}",
            self.start_timestamp.format("%d%b%Y %T"),
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
pub fn construct_data(
    url: String,
    res: reqwest::Response,
    start_timestamp: DateTime<Utc>,
) -> StatusData {
    StatusData {
        url,
        start_timestamp,
        end_timestamp: std::time::SystemTime::now().into(),
        healthy: res.status().is_success(),
        status: res.status(),
    }
}

pub fn construct_err(
    url: String,
    err: reqwest::Error,
    start_timestamp: DateTime<Utc>,
) -> StatusData {
    StatusData {
        url,
        start_timestamp,
        end_timestamp: std::time::SystemTime::now().into(),
        healthy: false,
        status: match err.status() {
            Some(status) => status,
            None => StatusCode::SERVICE_UNAVAILABLE,
        },
    }
}

/**
 * Ping the relevant URLs to see if they are up
 *
 * TODO: make the request non-blocking
 */
pub async fn ping_test() -> Vec<StatusData> {
    let client = reqwest::Client::new();

    let mut metrics: Vec<StatusData> = Vec::new();
    for url in URLS {
        let start_timestamp = std::time::SystemTime::now().into();
        let response = client.get((*url).to_string()).send().await;
        let metric = match response {
            Err(err) => construct_err(url.to_string(), err, start_timestamp),
            Ok(res) => construct_data(url.to_string(), res, start_timestamp),
        };

        metrics.push(metric);
    }

    metrics
}
