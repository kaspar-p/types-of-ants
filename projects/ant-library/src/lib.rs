use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use http::{header, HeaderValue};
use std::{env::set_var, fmt::Display};
use tracing::{debug, Level};
use tracing_subscriber::{fmt::writer::Tee, EnvFilter, FmtSubscriber};

pub mod db;
pub mod env;
pub mod find_up;
pub mod headers;
pub mod host_architecture;
pub mod manifest_file;
pub mod middleware;
pub mod routes;
pub mod sd;
pub mod secret;
pub mod services;

/// The standard ping that all typesofants web servers should use.
pub async fn api_ping() -> (StatusCode, String) {
    (StatusCode::OK, "healthy ant".to_string())
}

/// An API fallback function declaring which routes exist for the user to query.
pub fn api_fallback(routes: &[&str]) -> (StatusCode, String) {
    (
        StatusCode::NOT_FOUND,
        format!(
            "Unknown route. Valid routes are:\n{}",
            routes
                .iter()
                .map(|&r| String::from(r))
                .map(|r| { " -> ".to_owned() + &r + "\n" })
                .collect::<String>()
        ),
    )
}

pub fn set_global_logs(project: &str) -> () {
    // SAFETY: according to this discussion: https://users.rust-lang.org/t/unsafe-std-set-var-change/112704/68
    // this function is unsafe if-and-only-if there is FFI code modifying the environment lock that the
    // rust standard library sets. We don't use FFI!
    unsafe {
        set_var("RUST_LOG", "debug");
    }
    dotenv::dotenv().expect("Malformed or missing .env file");

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(false)
        .with_line_number(false)
        .with_env_filter(
            EnvFilter::try_from_env("RUST_LOG")
                .unwrap()
                .add_directive(format!("{}=debug", project).parse().unwrap())
                .add_directive("ant_library=debug".parse().unwrap())
                .add_directive("ant_data_farm=debug".parse().unwrap())
                .add_directive("glimmer=debug".parse().unwrap())
                .add_directive("tower_http=debug".parse().unwrap())
                .add_directive("axum::rejection=trace".parse().unwrap())
                .add_directive("tokio_cron_scheduler=debug".parse().unwrap())
                .add_directive("hyper=off".parse().unwrap()),
        )
        .with_ansi(false)
        .with_writer(Tee::new(
            std::io::stdout,
            tracing_appender::rolling::hourly("./logs", format!("{}.log", project)),
        ))
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    debug!("Logs initialized...");
}

#[derive(Debug)]
pub enum Mode {
    Dev,
    Prod,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Mode::Dev => f.write_str("DEV")?,
            Mode::Prod => f.write_str("PRODUCTION")?,
        };
        Ok(())
    }
}

pub fn get_mode() -> Mode {
    match dotenv::var("ANT_ON_THE_WEB_MODE") {
        Ok(mode) if mode.as_str() == "dev" => Mode::Dev,
        _ => Mode::Prod,
    }
}

pub async fn middleware_mode_headers(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut response = next.run(req).await;

    let response = match get_mode() {
        Mode::Dev => {
            let headers = response.headers_mut();
            headers.append(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_str("true").unwrap(),
            );
            response
        }
        Mode::Prod => response,
    };
    return Ok(response);
}
