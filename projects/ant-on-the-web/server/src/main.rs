use ant_data_farm::AntDataFarmClient;
use ant_library::get_mode;
use ant_on_the_web::{sms::Sms, state::InnerApiState};
use rand::SeedableRng;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tracing::debug;

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-on-the-web");

    debug!("Setting up state...");
    let state = InnerApiState {
        // None config for production use-case
        dao: Arc::new(
            AntDataFarmClient::new(None)
                .await
                .expect("Connected to db!"),
        ),

        // Twilio client for sending data.
        sms: Arc::new(Sms::new()),

        // Choose OS RNG to seed the std PRNG
        rng: Arc::new(Mutex::new(rand::rngs::StdRng::from_rng(&mut rand::rng()))),
    };
    let app = ant_on_the_web::make_routes(&state).expect("route init");

    let port: u16 = dotenv::var("ANT_ON_THE_WEB_PORT")
        .expect("ANT_ON_THE_WEB_PORT environment variable not found")
        .parse()
        .expect("ANT_ON_THE_WEB_PORT was not u16");

    debug!("Starting [{}] server on port [{}]...", get_mode(), port);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("Server failed!");
}
