use ant_data_farm::AntDataFarmClient;
use axum::{extract::State, Router};
use std::sync::Arc;

use crate::clients::sms::Sms;

#[derive(Clone)]
pub struct InnerApiState {
    pub dao: Arc<AntDataFarmClient>,
    pub sms: Arc<Sms>,
}

pub type ApiState = State<InnerApiState>;
pub type ApiRouter = Router<InnerApiState>;
