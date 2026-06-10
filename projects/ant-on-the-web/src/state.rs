use ant_data_farm::AntDataFarmClient;
use ant_library::routes::Routes;
use ant_library::sd::reader::ServiceDiscovery;
use axum::{
    extract::{FromRef, State},
    Router,
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

use crate::clients::{email::EmailSender, sms::SmsSender};

#[derive(Clone, FromRef)]
pub struct InnerApiState {
    /// Where static assets (JS, CSS) are stored.
    pub static_dir: PathBuf,

    pub sd: Arc<ServiceDiscovery>,
    pub dao: Arc<AntDataFarmClient>,
    pub sms: Arc<dyn SmsSender>,
    pub email: Arc<dyn EmailSender>,

    pub rng: Arc<Mutex<rand::rngs::StdRng>>,
}

pub type ApiState = State<InnerApiState>;
pub type ApiRouter = Router<InnerApiState>;
pub type ApiRoutes = Routes<InnerApiState>;
