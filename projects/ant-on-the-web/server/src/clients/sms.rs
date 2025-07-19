use std::any::Any;

use tracing::{debug, error, info};
use twilio::{Client, OutboundMessage, TwilioError};

#[async_trait::async_trait]
pub trait SmsSender: Any + Send + Sync {
    /// Send an SMS message to `to_phone` with the `content` as body. Return a unique identifier
    /// for the message if there is one, likely from the sms provider.
    async fn send_msg(&self, to_phone: &str, content: &str) -> Result<String, SmsError>;
}

pub struct Sms {
    source_phone: String,

    client: Client,
}

pub enum SmsError {
    InternalServerError(anyhow::Error),
    BadPhoneNumber,
}

impl Sms {
    pub fn new() -> Self {
        Sms {
            source_phone: ant_library::secret::load_secret("twilio_phone_number").unwrap(),
            client: twilio::Client::new(
                ant_library::secret::load_secret("twilio_account_id")
                    .unwrap()
                    .as_str(),
                ant_library::secret::load_secret("twilio_auth_token")
                    .unwrap()
                    .as_str(),
            ),
        }
    }
}

#[async_trait::async_trait]
impl SmsSender for Sms {
    async fn send_msg(&self, to_phone: &str, content: &str) -> Result<String, SmsError> {
        debug!("Sending SMS {to_phone}: {content}");
        let msg = self
            .client
            .send_message(OutboundMessage {
                from: &self.source_phone,
                to: "+19704812142",
                body: "hello",
            })
            .await;
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => match e {
                TwilioError::BadRequest => {
                    info!("sending sms to that number failed");
                    return Err(SmsError::BadPhoneNumber);
                }
                _ => {
                    error!("sending sms failed: {}", e);
                    return Err(SmsError::InternalServerError(anyhow::anyhow!(e)));
                }
            },
        };

        debug!("Sent SMS {:?}", msg);

        Ok(msg.sid)
    }
}
