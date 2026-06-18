use ant_data_farm::AntDataFarmClient;
use ant_library::{get_mode, sd::reader::ServiceDiscovery};
use ant_library::rng::SystemRng;
use ant_on_the_web::{email::MailjetEmailSender, sms::Sms, state::InnerApiState, ApiOptions};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tracing::debug;

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-on-the-web");

    let sd = ServiceDiscovery::new(
        dotenv::var("ANT_MATCHMAKER_HTTP_PORT")
            .expect("No ANT_MATCHMAKER_HTTP_PORT variable!")
            .parse::<u16>()
            .expect("ANT_MATCHMAKER_HTTP_PORT was not u16!"),
    );

    let dao = AntDataFarmClient::connect_discovered(Arc::new(sd.clone()), vec![])
        .await
        .expect("db connection failed");

    debug!("Setting up state...");
    let state = InnerApiState {
        // Static files all stored locally in ./static
        static_dir: PathBuf::from("./static"),

        sd: Arc::new(sd),
        dao: Arc::new(dao),

        // Twilio client for sending texts
        sms: Arc::new(Sms::new()),

        // Mailjet client for sending emails
        email: Arc::new(MailjetEmailSender::new()),

        rng: Arc::new(SystemRng),
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
