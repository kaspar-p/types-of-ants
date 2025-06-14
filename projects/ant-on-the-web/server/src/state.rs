use ant_data_farm::AntDataFarmClient;
use axum::{extract::State, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::clients::sms::SmsSender;

#[derive(Clone)]
pub struct InnerApiState {
    pub dao: Arc<AntDataFarmClient>,
    pub sms: Arc<dyn SmsSender>,
    pub rng: Arc<Mutex<rand::rngs::StdRng>>,
}

pub type ApiState = State<InnerApiState>;
pub type ApiRouter = Router<InnerApiState>;
