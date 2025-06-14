use std::any::Any;

use tracing::debug;
use twilio::{Client, OutboundMessage};

#[async_trait::async_trait]
pub trait SmsSender: Any + Send + Sync {
    /// Send an SMS message to `to_phone` with the `content` as body. Return a unique identifier
    /// for the message if there is one, likely from the sms provider.
    async fn send_msg(&self, to_phone: &str, content: &str) -> Result<String, anyhow::Error>;
}

pub struct Sms {
    source_phone: String,

    client: Client,
}

pub enum SmsError {
    InternalServerError(twilio::TwilioError),
}

impl Sms {
    pub fn new() -> Self {
        Sms {
            source_phone: dotenv::var("TWILIO_PHONE_NUMBER").unwrap(),
            client: twilio::Client::new(
                dotenv::var("TWILIO_ACCOUNT_ID").unwrap().as_str(),
                dotenv::var("TWILIO_AUTH_TOKEN").unwrap().as_str(),
            ),
        }
    }
}

#[async_trait::async_trait]
impl SmsSender for Sms {
    async fn send_msg(&self, to_phone: &str, content: &str) -> Result<String, anyhow::Error> {
        debug!("Sending SMS {to_phone} ::: {content}");
        let msg = self
            .client
            .send_message(OutboundMessage {
                from: &self.source_phone,
                to: to_phone,
                body: content,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        debug!("Sent SMS {:?}", msg);

        Ok(msg.sid)
    }
}
