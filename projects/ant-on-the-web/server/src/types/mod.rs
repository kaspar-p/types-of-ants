use ant_data_farm::AntDataFarmClient;
use axum::{extract::State, Router};
use std::sync::Arc;

pub type DbRouter = Router<Arc<AntDataFarmClient>>;
pub type DbState = State<Arc<AntDataFarmClient>>;
