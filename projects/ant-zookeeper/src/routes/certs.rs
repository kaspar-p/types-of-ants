use std::time::Duration;

use acme_lib::{persist::FilePersist, Directory};
use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use http::StatusCode;
use rsa::{pkcs1::EncodeRsaPrivateKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use tokio::{fs::create_dir_all, time::sleep};
use tracing::{debug, info};

use crate::{err::AntZookeeperError, state::AntZookeeperState};

fn acme_domain(owned_domain: &str) -> String {
    format!("_acme-challenge.{owned_domain}")
}

async fn clean_acme_records(
    state: &AntZookeeperState,
    domains: &Vec<String>,
) -> Result<bool, AntZookeeperError> {
    // Clean old ACME records because LetsEncrypt won't validate if there is more than 1 record there...
    let mut cleaned = false;
    for domain in domains {
        let records = state
            .dns
            .lock()
            .await
            .list_txt_records(&acme_domain(domain))
            .await?;

        for record in records {
            info!("Cleaning old ACME related record: {domain} => {record:?}");
            state.dns.lock().await.delete_txt_record(&record.id).await?;
            cleaned = true;
        }
    }

    return Ok(cleaned);
}

#[derive(Serialize, Deserialize)]
pub struct ProvisionCertificateRequest {
    pub domains: Vec<String>,
}

async fn provision_certificate(
    State(state): State<AntZookeeperState>,
    Json(req): Json<ProvisionCertificateRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if req.domains.is_empty() {
        return Ok((StatusCode::BAD_REQUEST, "No domains requested.".to_string()));
    }

    let persist_dir = state.root_dir.join("certs-db");
    create_dir_all(&persist_dir).await?;
    let persist = FilePersist::new(persist_dir);

    let dir = Directory::from_url(persist, state.acme_url.clone())?;

    let acc = dir.account(&state.acme_contact_email)?;

    // Clean old ACME records because LetsEncrypt won't validate if there is more than 1 record there...
    let cleaned = clean_acme_records(&state, &req.domains).await?;
    if cleaned {
        info!("Sleeping for some time before continuing to let DNS records propagate...");
        sleep(Duration::from_secs(10)).await;
        info!("Continuing!");
    }

    info!("Creating new order...");
    let mut order = acc.new_order(
        &req.domains[0],
        &req.domains[1..]
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>(),
    )?;

    if order.is_validated() {
        info!("Order confirmed to be valid based on certificate store, returning early...");
        return Ok((StatusCode::OK, "Certificate provisioned.".to_string()));
    }

    let csr = loop {
        if let Some(csr) = order.confirm_validations() {
            info!("Challenge met, moving to providing private key material...");
            break csr;
        }

        let auths = order.authorizations()?;

        for auth in auths {
            let challenge = auth.dns_challenge();

            info!("Putting DNS record...");
            state
                .dns
                .lock()
                .await
                .put_txt_record(&acme_domain(&auth.domain_name()), challenge.dns_proof())
                .await?;

            loop {
                info!("Polling dns for presence of record...");
                if state
                    .dns
                    .lock()
                    .await
                    .list_txt_records(&acme_domain(&auth.domain_name()))
                    .await?
                    .iter()
                    .find(|record| record.content == challenge.dns_proof())
                    .is_some()
                {
                    break;
                }

                sleep(Duration::from_millis(500)).await;
            }

            info!("DNS records input, sleeping for propagation...");
            sleep(Duration::from_millis(30_000)).await;

            info!("Asking ACME to check...");
            challenge.validate(5_000)?;
            order.refresh()?;
        }
    };

    debug!("Generating private key...");
    let mut rng = state.rng;
    let priv_key = RsaPrivateKey::new(&mut rng, 2048)?;

    debug!("Generating PEM document...");
    let pem = priv_key.to_pkcs1_pem(base64ct::LineEnding::LF)?;

    debug!("Finalizing order...");
    let order_certificate = csr.finalize(&pem, 5_000)?;

    debug!("Downloading certificate...");
    let files = order_certificate.download_and_save_cert()?;

    debug!("Deleting TXT records created during the challenge...");
    clean_acme_records(&state, &req.domains).await?;

    return Ok((StatusCode::OK, "Certificate provisioned.".to_string()));
}

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new().route_with_tsr("/cert", post(provision_certificate))
}
