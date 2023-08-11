use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Host {
    pub label: String,
    pub ip: String,
}

impl Host {
    pub fn new() -> Host {
        Host {
            // Terrible practice, the IP of my Raspberry Pi
            label: String::from("Kaspar's Raspberry Pi"),
            ip: "192.168.10.162".to_owned(),
        }
    }
}
