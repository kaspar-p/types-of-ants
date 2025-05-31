use tracing::debug;
use twilio::{Client, OutboundMessage};

pub struct Sms {
    source_phone: String,
    is_dry_run: bool,

    client: Client,
}

pub enum SmsError {
    InternalServerError(twilio::TwilioError),
}

impl Sms {
    pub fn new(is_dry_run: bool) -> Self {
        Sms {
            is_dry_run,
            source_phone: dotenv::var("TWILIO_PHONE_NUMBER").unwrap(),
            client: twilio::Client::new(
                dotenv::var("TWILIO_ACCOUNT_ID").unwrap().as_str(),
                dotenv::var("TWILIO_AUTH_TOKEN").unwrap().as_str(),
            ),
        }
    }

    pub async fn send_msg(&self, to_phone: &str, content: &str) -> Result<String, SmsError> {
        if self.is_dry_run {
            return Ok("send-id".to_string());
        }

        debug!("Sending SMS {to_phone} ::: {content}");
        let msg = self
            .client
            .send_message(OutboundMessage {
                from: &self.source_phone,
                to: to_phone,
                body: content,
            })
            .await
            .map_err(|e| SmsError::InternalServerError(e))?;
        debug!("Sent SMS {:?}", msg);

        Ok(msg.sid)
    }
}
