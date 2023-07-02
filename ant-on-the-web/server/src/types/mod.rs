use ant_data_farm::Dao;
use axum::{extract::State, Router};
use std::sync::Arc;

pub type DaoRouter = Router<Arc<Dao>>;
pub type DaoState = State<Arc<Dao>>;
