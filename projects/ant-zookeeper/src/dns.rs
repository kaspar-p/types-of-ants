use cloudflare::{
    endpoints::dns::dns::{self, DnsContent},
    framework::{
        auth::Credentials,
        client::{async_api::Client, ClientConfig},
    },
};
use tracing::{debug, warn};

#[async_trait::async_trait]
pub trait Dns: Send {
    /// Add a TXT record for `domain`, with value `val`. Returns that identifier.
    /// Idempotent for identical records added onto the same domain.
    async fn put_txt_record(&self, domain: &str, val: String) -> Result<TxtRecord, anyhow::Error>;

    /// Delete a certain record directly by the identifier.
    async fn delete_txt_record(&self, record_id: &str) -> Result<(), anyhow::Error>;

    /// List all TXT records in the domain.
    async fn list_txt_records(&self, domain: &str) -> Result<Vec<TxtRecord>, anyhow::Error>;
}

pub struct CloudFlareDns {
    zone_id: String,
    cloudflare: Client,
}

impl CloudFlareDns {
    pub fn new(user_auth_token: String, zone_id: String) -> Self {
        Self {
            zone_id,
            cloudflare: Client::new(
                Credentials::UserAuthToken {
                    token: user_auth_token,
                },
                ClientConfig::default(),
                cloudflare::framework::Environment::Production,
            )
            .unwrap(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TxtRecord {
    pub id: String,
    pub content: String,
}

#[async_trait::async_trait]
impl Dns for CloudFlareDns {
    async fn put_txt_record(&self, domain: &str, val: String) -> Result<TxtRecord, anyhow::Error> {
        if let Some(identical_record) = self
            .list_txt_records(domain)
            .await?
            .iter()
            .find(|record| record.content == val)
        {
            warn!("Identical record already present, short-circuiting: {identical_record:?}");
            return Ok(identical_record.clone());
        }

        let request = dns::CreateDnsRecord {
            zone_identifier: &self.zone_id,
            params: dns::CreateDnsRecordParams {
                name: &domain,
                content: DnsContent::TXT { content: val },
                ttl: Some(1), // automatic
                priority: None,
                proxied: Some(false),
            },
        };

        debug!("cloudflare:CreateDnsRecord:request {request:?}");
        let response = self.cloudflare.request(&request).await?;
        debug!("cloudflare:CreateDnsRecord:response {response:?}");

        Ok(TxtRecord {
            id: response.result.id,
            content: match response.result.content {
                DnsContent::TXT { content } => content,
                _ => panic!("Cannot be type other than TXT."),
            },
        })
    }

    async fn delete_txt_record(&self, record_id: &str) -> Result<(), anyhow::Error> {
        let request = dns::DeleteDnsRecord {
            zone_identifier: &self.zone_id,
            identifier: &record_id,
        };

        debug!("cloudflare:DeleteDnsRecord:request {request:?}");
        let response = self.cloudflare.request(&request).await?;
        debug!("cloudflare:DeleteDnsRecord:response {response:?}");

        assert_eq!(response.result.id, record_id);

        Ok(())
    }

    async fn list_txt_records(&self, domain: &str) -> Result<Vec<TxtRecord>, anyhow::Error> {
        let request = dns::ListDnsRecords {
            zone_identifier: &self.zone_id,
            params: dns::ListDnsRecordsParams {
                name: Some(domain.to_string()),
                ..Default::default()
            },
        };

        debug!("cloudflare:ListDnsRecords:request {request:?}");
        let response = self.cloudflare.request(&request).await?;
        debug!("cloudflare:ListDnsRecords:response {response:?}");

        let txt_records = response
            .result
            .iter()
            .filter_map(|record| match &record.content {
                DnsContent::TXT { content } => Some(TxtRecord {
                    id: record.id.clone(),
                    content: content.clone(),
                }),
                _ => None,
            })
            .collect();

        Ok(txt_records)
    }
}
