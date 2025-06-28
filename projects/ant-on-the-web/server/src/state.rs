use ant_data_farm::AntDataFarmClient;
use axum::{extract::State, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::clients::{email::EmailSender, sms::SmsSender};

#[derive(Clone)]
pub struct InnerApiState {
    pub dao: Arc<AntDataFarmClient>,
    pub sms: Arc<dyn SmsSender>,
    pub email: Arc<dyn EmailSender>,
    pub rng: Arc<Mutex<rand::rngs::StdRng>>,
}

pub type ApiState = State<InnerApiState>;
pub type ApiRouter = Router<InnerApiState>;
