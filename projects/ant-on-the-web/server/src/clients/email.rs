use std::any::Any;

use tracing::{error, info};

pub enum EmailError {
    InternalServerError(anyhow::Error),
}

#[async_trait::async_trait]
pub trait EmailSender: Any + Send + Sync {
    /// Send a single email to `recipient_email` with the subject and content as desired.
    async fn send_email(
        &self,
        recipient_email: &str,
        subject: String,
        content: String,
    ) -> Result<String, EmailError>;
}

pub struct MailjetEmailSender {
    client: mailjet_rs::Client,
    from_email: String,
    from_name: String,
}

impl MailjetEmailSender {
    pub fn new() -> Self {
        let public_key = dotenv::var("MAILJET_API_KEY").expect("No MAILJET_API_KEY.");
        let private_key = dotenv::var("MAILJET_SECRET_KEY").expect("No MAILJET_SECRET_KEY.");
        let sender_email = dotenv::var("MAILJET_SENDER_EMAIL").expect("No MAILJET_SENDER_EMAIL.");
        let sender_from = dotenv::var("MAILJET_SENDER_FROM").expect("No MAILJET_SENDER_FROM.");

        Self {
            client: mailjet_rs::Client::new(
                mailjet_rs::SendAPIVersion::V3,
                &public_key,
                &private_key,
            ),
            from_email: sender_email,
            from_name: sender_from,
        }
    }
}

#[async_trait::async_trait]
impl EmailSender for MailjetEmailSender {
    async fn send_email(
        &self,
        recipient_email: &str,
        subject: String,
        content: String,
    ) -> Result<String, EmailError> {
        let mut msg = mailjet_rs::v3::Message::new(
            &self.from_email,
            &self.from_name,
            Some(subject),
            Some(content),
        );
        msg.push_recipient(mailjet_rs::common::Recipient::new(recipient_email));

        let res = self
            .client
            .send(msg)
            .await
            .map_err(|e| EmailError::InternalServerError(anyhow::Error::msg(e.message)))?;

        let sent = res.sent.first();
        match sent {
            None => {
                error!("No sent response arrived from MailJet: {:?}", res);
                return Err(EmailError::InternalServerError(anyhow::Error::msg(
                    "No sent response arrived from MailJet",
                )));
            }
            Some(s) => {
                info!("Sent email: {:?}", s);
                return Ok(s.message_uuid.clone());
            }
        }
    }
}
