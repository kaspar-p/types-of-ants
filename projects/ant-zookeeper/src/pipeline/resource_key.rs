use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use strum::EnumCount;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier(String);

impl Identifier {
    pub fn new(value: impl Into<String>) -> Result<Self, anyhow::Error> {
        let s = value.into();
        if s.is_empty() {
            anyhow::bail!("Identifier must not be empty");
        }
        if s.contains(':') {
            anyhow::bail!("Identifier must not contain ':': {s}");
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Identifier {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Serialize for Identifier {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumCount)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeploymentResource {
    /// The running service on a specific host.
    HostService {
        host_id: Identifier,
        service_id: Identifier,
    },
    /// Database schema migration for a service in an environment.
    DatabaseMigration {
        host: Identifier,
        service_id: Identifier,
        environment: Identifier,
    },
    /// nginx routing config on the gateway for an environment.
    GatewayRouting { environment: Identifier },
    /// Prometheus alert rules for a service.
    AlertRules { service_id: Identifier },
    /// Promtail log pipeline rules for a service on a specific host.
    LogRules {
        host_id: Identifier,
        service_id: Identifier,
    },
}

impl Display for DeploymentResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentResource::HostService {
                host_id,
                service_id,
            } => write!(f, "host_service:{host_id}:{service_id}"),
            DeploymentResource::DatabaseMigration {
                service_id,
                host,
                environment,
            } => write!(f, "database_migration:{service_id}:{host}:{environment}"),
            DeploymentResource::GatewayRouting { environment } => {
                write!(f, "gateway_routing:{environment}")
            }
            DeploymentResource::AlertRules { service_id } => {
                write!(f, "alert_rules:{service_id}")
            }
            DeploymentResource::LogRules {
                host_id,
                service_id,
            } => write!(f, "log_rules:{host_id}:{service_id}"),
        }
    }
}

impl FromStr for DeploymentResource {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() < 2 {
            anyhow::bail!("DeploymentResource missing type prefix: {s}");
        }

        let (variant, rest) = (parts[0], parts[1]);

        match variant {
            "host_service" => {
                let fields: Vec<&str> = rest.splitn(2, ':').collect();
                if fields.len() != 2 {
                    anyhow::bail!("host_service requires host_id:service_id, got: {rest}");
                }
                Ok(DeploymentResource::HostService {
                    host_id: Identifier::new(fields[0])?,
                    service_id: Identifier::new(fields[1])?,
                })
            }
            "database_migration" => {
                let fields: Vec<&str> = rest.splitn(2, ':').collect();
                if fields.len() != 2 {
                    anyhow::bail!(
                        "database_migration requires service_id:environment, got: {rest}"
                    );
                }
                Ok(DeploymentResource::DatabaseMigration {
                    service_id: Identifier::new(fields[0])?,
                    host: Identifier::new(fields[1])?,
                    environment: Identifier::new(fields[2])?,
                })
            }
            "gateway_routing" => Ok(DeploymentResource::GatewayRouting {
                environment: Identifier::new(rest)?,
            }),
            "alert_rules" => Ok(DeploymentResource::AlertRules {
                service_id: Identifier::new(rest)?,
            }),
            "log_rules" => {
                let fields: Vec<&str> = rest.splitn(2, ':').collect();
                if fields.len() != 2 {
                    anyhow::bail!("log_rules requires host_id:service_id, got: {rest}");
                }
                Ok(DeploymentResource::LogRules {
                    host_id: Identifier::new(fields[0])?,
                    service_id: Identifier::new(fields[1])?,
                })
            }
            _ => anyhow::bail!("Unknown DeploymentResource variant: {variant}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn id(s: &str) -> Identifier {
        Identifier::new(s).unwrap()
    }

    fn all_variants() -> Vec<DeploymentResource> {
        vec![
            DeploymentResource::HostService {
                host_id: id("antworker002"),
                service_id: id("ant-on-the-web"),
            },
            DeploymentResource::DatabaseMigration {
                service_id: id("ant-on-the-web-users-db"),
                host: id("antworker002"),
                environment: id("beta"),
            },
            DeploymentResource::GatewayRouting {
                environment: id("prod"),
            },
            DeploymentResource::AlertRules {
                service_id: id("ant-on-the-web"),
            },
            DeploymentResource::LogRules {
                host_id: id("antworker002"),
                service_id: id("ant-on-the-web"),
            },
        ]
    }

    #[test]
    fn identifier_rejects_empty() {
        assert!(Identifier::new("").is_err());
    }

    #[test]
    fn identifier_rejects_colon() {
        assert!(Identifier::new("bad:value").is_err());
    }

    #[test]
    fn identifier_accepts_valid() {
        assert!(Identifier::new("ant-on-the-web").is_ok());
        assert!(Identifier::new("antworker002").is_ok());
        assert!(Identifier::new("prod").is_ok());
    }

    #[test]
    fn roundtrip_all_variants() {
        let keys = all_variants();

        for key in &keys {
            let s = key.to_string();
            let parsed: DeploymentResource = s.parse().unwrap();
            assert_eq!(&parsed, key, "Roundtrip failed for: {s}");
        }

        let unique: HashSet<_> = keys.iter().map(std::mem::discriminant).collect();
        assert_eq!(unique.len(), DeploymentResource::COUNT);
    }

    #[test]
    fn all_variants_produce_unique_strings() {
        let keys = all_variants();

        let strings: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        let unique: HashSet<&String> = strings.iter().collect();
        assert_eq!(
            strings.len(),
            unique.len(),
            "Duplicate keys found: {:?}",
            strings
        );
    }

    #[test]
    fn same_variant_different_fields_unique() {
        let keys = vec![
            DeploymentResource::HostService {
                host_id: id("antworker001"),
                service_id: id("ant-on-the-web"),
            },
            DeploymentResource::HostService {
                host_id: id("antworker002"),
                service_id: id("ant-on-the-web"),
            },
            DeploymentResource::HostService {
                host_id: id("antworker001"),
                service_id: id("ant-gateway"),
            },
        ];

        let strings: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        let unique: HashSet<&String> = strings.iter().collect();
        assert_eq!(
            strings.len(),
            unique.len(),
            "Duplicate keys found: {:?}",
            strings
        );
    }

    #[test]
    fn from_str_rejects_unknown_variant() {
        assert!("unknown:foo:bar".parse::<DeploymentResource>().is_err());
    }

    #[test]
    fn from_str_rejects_missing_fields() {
        assert!("host_service:only-one-field"
            .parse::<DeploymentResource>()
            .is_err());
        assert!("database_migration:only-one"
            .parse::<DeploymentResource>()
            .is_err());
        assert!("log_rules:only-one".parse::<DeploymentResource>().is_err());
    }

    #[test]
    fn from_str_rejects_empty_fields() {
        assert!("host_service::ant-on-the-web"
            .parse::<DeploymentResource>()
            .is_err());
        assert!("alert_rules:".parse::<DeploymentResource>().is_err());
    }

    #[test]
    fn serde_json_roundtrip() {
        for key in all_variants() {
            let json = serde_json::to_string(&key).unwrap();
            let parsed: DeploymentResource = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, key, "Serde roundtrip failed for: {json}");
        }
    }

    #[test]
    fn serde_json_rejects_colon_in_identifier() {
        let json = r#"{"type":"host_service","host_id":"bad:id","service_id":"ok"}"#;
        assert!(serde_json::from_str::<DeploymentResource>(json).is_err());
    }
}
