use serde::Deserialize;

/// Created from:
/// https://www.twilio.com/docs/messaging/guides/webhook-request
#[derive(Debug, Deserialize)]
pub struct TwilioWebhookMessage {
    #[serde(rename(deserialize = "ToCountry"))]
    pub to_country: String,

    #[serde(rename(deserialize = "ToState"))]
    pub to_state: String,

    #[serde(rename(deserialize = "ToCity"))]
    pub to_city: String,

    #[serde(rename(deserialize = "FromCountry"))]
    pub from_country: String,

    #[serde(rename(deserialize = "FromState"))]
    pub from_state: String,

    #[serde(rename(deserialize = "FromCity"))]
    pub from_city: String,

    #[serde(rename(deserialize = "FromZip"))]
    pub from_zip: String,

    #[serde(rename(deserialize = "SmsStatus"))]
    pub sms_status: String,

    #[serde(rename(deserialize = "NumSegments"))]
    pub num_segments: i32,

    #[serde(rename(deserialize = "MessageSid"))]
    pub message_sid: String,

    #[serde(rename(deserialize = "ApiVersion"))]
    pub api_version: String,

    #[serde(rename(deserialize = "SmsMessageSid"))]
    pub sms_message_sid: String,

    #[serde(rename(deserialize = "SmsSid"))]
    pub sms_sid: String,

    #[serde(rename(deserialize = "AccountSid"))]
    pub account_sid: String,

    #[serde(rename(deserialize = "MessagingServiceSid"))]
    pub messaging_service_sid: String,

    #[serde(rename(deserialize = "From"))]
    pub from: String,

    #[serde(rename(deserialize = "To"))]
    pub to: String,

    #[serde(rename(deserialize = "Body"))]
    pub body: String,

    #[serde(rename(deserialize = "NumMedia"))]
    pub num_media: String,
}
