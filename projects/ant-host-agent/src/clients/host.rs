use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Host {
    pub label: String,
    pub port: u16,
    pub hostname: String,
}

impl Host {
    pub fn new(hostname: String, port: u16) -> Host {
        Host {
            label: format!("ANT_WORKER: {}:{}", hostname, port.to_string()),
            port,
            hostname,
        }
    }

    pub fn http_endpoint(&self) -> String {
        format!("http://{}:{}", self.hostname, self.port)
    }
}
