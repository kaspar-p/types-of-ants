use std::str::FromStr;

use serde::Serialize;

#[derive(
    PartialEq, Eq, Hash, Debug, Clone, Serialize, serde_with::DeserializeFromStr, PartialOrd, Ord,
)]
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

    pub fn rust_target(&self) -> &str {
        match &self {
            Self::Aarch64 => "aarch64-unknown-linux-gnu",
            Self::ArmV7 => "armv7-unknown-linux-gnueabihf",
            Self::X86_64 => "x86_64-unknown-linux-gnu",
        }
    }

    pub fn docker_platform(&self) -> &str {
        match &self {
            Self::ArmV7 | Self::Aarch64 => "linux/arm64",
            Self::X86_64 => "linux/amd64",
        }
    }

    pub fn prometheus_os(&self) -> &str {
        match &self {
            Self::ArmV7 | Self::Aarch64 => "linux",
            Self::X86_64 => "linux", // TODO: ?
        }
    }

    pub fn prometheus_arch(&self) -> &str {
        match &self {
            Self::ArmV7 => "armv7",
            Self::Aarch64 => "arm64",
            Self::X86_64 => "amd64",
        }
    }
}

impl FromStr for HostArchitecture {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "amd64" | "x86" | "x86_64" => HostArchitecture::X86_64,
            "arm" | "arm64" | "aarch64" => HostArchitecture::Aarch64,
            "raspbian" | "armv7" | "armV7" => HostArchitecture::ArmV7,
            _ => return Err(anyhow::Error::msg(format!("unsupported architecture: {s}"))),
        })
    }
}

impl ToString for HostArchitecture {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

#[cfg(test)]
mod test {
    use crate::host_architecture::HostArchitecture;

    #[test]
    fn deserializes_nonstandard_arches() {
        assert_eq!(
            serde_json::from_str::<HostArchitecture>("\"arm64\"").unwrap(),
            HostArchitecture::Aarch64
        );
        assert_eq!(
            serde_json::from_str::<HostArchitecture>("\"armv7\"").unwrap(),
            HostArchitecture::ArmV7
        );
        assert_eq!(
            serde_json::from_str::<HostArchitecture>("\"armV7\"").unwrap(),
            HostArchitecture::ArmV7
        );
        assert_eq!(
            serde_json::from_str::<HostArchitecture>("\"raspbian\"").unwrap(),
            HostArchitecture::ArmV7
        );
    }
}
