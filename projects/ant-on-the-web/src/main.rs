use ant_data_farm::AntDataFarmClient;
use ant_library::get_mode;
use ant_on_the_web::{email::MailjetEmailSender, sms::Sms, state::InnerApiState, ApiOptions};
use rand::SeedableRng;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tracing::debug;

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-on-the-web");

    debug!("Setting up state...");
    let state = InnerApiState {
        // Static files all stored locally in ./static
        static_dir: PathBuf::from("./static"),

        // None config for production use-case
        dao: Arc::new(
            AntDataFarmClient::connect_from_env(None)
                .await
                .expect("db connection failed"),
        ),

        // Twilio client for sending texts
        sms: Arc::new(Sms::new()),

        // Mailjet client for sending emails
        email: Arc::new(MailjetEmailSender::new()),

        // Choose OS RNG to seed the std PRNG
        rng: Arc::new(Mutex::new(rand::rngs::StdRng::from_rng(&mut rand::rng()))),
    };
    let app =
        ant_on_the_web::make_routes(&state, ApiOptions { tps: 250 }).expect("route init failed");

    let port: u16 = dotenv::var("ANT_ON_THE_WEB_PORT")
        .expect("ANT_ON_THE_WEB_PORT environment variable not found")
        .parse()
        .expect("ANT_ON_THE_WEB_PORT was not u16");

    debug!("Starting [{}] server on port [{}]...", get_mode(), port);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("server failed");
}
