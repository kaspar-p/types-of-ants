use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostArchitecture {
    X86_64,
    Aarch64,
    ArmV7,
}

impl HostArchitecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            HostArchitecture::Aarch64 => "aarch64",
            HostArchitecture::ArmV7 => "armv7",
            HostArchitecture::X86_64 => "x86_64",
        }
    }
}

impl FromStr for HostArchitecture {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "amd64" | "x86" | "x86_64" => HostArchitecture::X86_64,
            "arm" | "arm64" | "aarch64" => HostArchitecture::Aarch64,
            "raspbian" | "armv7" => HostArchitecture::ArmV7,
            _ => return Err(anyhow::anyhow!("unsupported architecture: {s}")),
        })
    }
}

impl ToString for HostArchitecture {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}
