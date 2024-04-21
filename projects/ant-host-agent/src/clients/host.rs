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

    pub fn http_endpoint(&self, path: Option<String>) -> String {
        match path {
            None => format!("http://{}:{}", self.hostname, self.port),
            Some(p) => format!("http://{}:{}/{}", self.hostname, self.port, p),
        }
    }
}
