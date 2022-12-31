use postgres;
use reqwest::{blocking::Response, StatusCode};

struct StatusData {
    healthy: bool,
}

/**
 * From a web response, construct data to go into a database
 */
fn construct_data(url: &str, res: Response) -> StatusData {
    let status = res.status();
    if status.is_success() {
        println!("[{}] Successful {}!", url, status.as_str());
    } else if status.is_server_error() | status.is_client_error() {
        println!("[{}] Site down with {}!", url, status.as_str());
    }

    return StatusData {
        healthy: status.is_success(),
    };
}

/**
 * Emit data into a database
 */
pub fn emit_data(url: &str, res: Response) -> () {
    // Construct the data from the response
    let data: StatusData = construct_data(url, res);

    // Emit that data into a DB
}
