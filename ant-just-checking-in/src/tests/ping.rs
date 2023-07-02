use chrono::{DateTime, Utc};
use reqwest::StatusCode;

const URLS: &[&str] = &[
    "http://typesofants.org",
    "https://typesofants.org",
    "http://beta.typesofants.org",
    "https://beta.typesofants.org",
    "http://www.typesofants.org",
    "https://www.typesofants.org",
    "http://6krill.com",
    "https://6krill.com",
];

#[derive(Debug)]
pub struct StatusData {
    url: &'static str,
    start_timestamp: DateTime<Utc>,
    end_timestamp: DateTime<Utc>,
    healthy: bool,
    status: reqwest::StatusCode,
}

impl StatusData {
    pub fn to_test_sql_row(&self, test_id: i32) -> String {
        let healthy_boolean = if self.healthy { "TRUE" } else { "FALSE" };
        format!(
            "({}, '{}', '{}', '{}')",
            test_id, self.start_timestamp, self.end_timestamp, healthy_boolean
        )
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
fn construct_data(
    url: &'static str,
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

fn construct_err(
    url: &'static str,
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
            Err(err) => construct_err(url, err, start_timestamp),
            Ok(res) => construct_data(url, res, start_timestamp),
        };

        metrics.push(metric);
    }

    metrics
}
