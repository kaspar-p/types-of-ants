use ant_data_farm::AntDataFarmClient;
use ant_library::routes::Routes;
use ant_library::sd::reader::ServiceDiscovery;
use axum::{
    extract::{FromRef, State},
    Router,
};
use std::{path::PathBuf, sync::Arc};
use ant_library::{clock::Clock, rng::Rng};

use crate::clients::{email::EmailSender, sms::SmsSender};

#[derive(Clone, FromRef)]
pub struct InnerApiState {
    /// Where static assets (JS, CSS) are stored.
    pub static_dir: PathBuf,

    pub sd: Arc<ServiceDiscovery>,
    pub dao: Arc<AntDataFarmClient>,
    pub sms: Arc<dyn SmsSender>,
    pub email: Arc<dyn EmailSender>,

    pub rng: Arc<dyn Rng>,
    pub clock: Arc<dyn Clock>,
}

pub type ApiState = State<InnerApiState>;
pub type ApiRouter = Router<InnerApiState>;
pub type ApiRoutes = Routes<InnerApiState>;
