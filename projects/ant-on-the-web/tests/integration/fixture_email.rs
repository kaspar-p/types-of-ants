use std::sync::Arc;

use ant_on_the_web::email::{EmailError, EmailSender};
use tokio::sync::Mutex;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestEmail {
    pub recipient: String,
    pub subject: String,
    pub content: String,
}

pub struct TestEmailSender {
    pub sent: Arc<Mutex<Vec<TestEmail>>>,
}

impl TestEmailSender {
    pub fn new() -> Self {
        Self {
            sent: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn all_msgs(&self) -> Vec<TestEmail> {
        let msgs = self.sent.lock().await;

        return msgs.iter().map(|m| m.clone()).collect::<Vec<TestEmail>>();
    }
}

#[async_trait::async_trait]
impl EmailSender for TestEmailSender {
    async fn send_email(
        &self,
        recipient_email: &str,
        subject: String,
        content: String,
    ) -> Result<String, EmailError> {
        self.sent.lock().await.push(TestEmail {
            recipient: recipient_email.to_string(),
            subject,
            content,
        });

        return Ok("email-id".to_string());
    }
}
