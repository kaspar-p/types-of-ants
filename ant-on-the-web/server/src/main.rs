mod dao;
mod lib;
mod routes;

use axum::{
    http::{header::CONTENT_TYPE, Method},
    Router,
};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use dao::dao::Dao;
use dotenv;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tokio_postgres::NoTls;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

async fn database_connection() -> Result<Pool<PostgresConnectionManager<NoTls>>, dotenv::Error> {
    if let Err(e) = dotenv::dotenv() {
        panic!("Failed to load environment variables: {}", e);
    }

    let db_port = dotenv::var("DB_PG_PORT")?;
    let db_name = dotenv::var("DB_PG_NAME")?;
    let user = dotenv::var("DB_PG_USER")?;
    let pw = dotenv::var("DB_PG_PASSWORD")?;

    let connection_string = format!(
        "postgresql://{}:{}@localhost:{}/{}",
        user, pw, db_port, db_name
    );

    let manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> =
        Pool::builder().build(manager).await.unwrap();

    Ok(pool)
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    println!("Setting up database connection pool...");
    let pool = database_connection()
        .await
        .unwrap_or_else(|e| panic!("Failed to get environment variable: {}", e));

    println!("Initializing data access layer...");
    let dao = Arc::new(Dao::new(pool).await);

    let app = Router::new()
        .nest("/api/ants", routes::ants::router())
        .nest("/api/users", routes::users::router())
        .nest("/api/tests", routes::tests::router())
        .nest("/api/metrics", routes::metrics::router())
        .nest("/api/deployments", routes::deployments::router())
        .nest("/api/hosts", routes::hosts::router())
        .with_state(dao)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        )
        .nest_service("/", ServeDir::new("static"));

    println!("Starting server...");
    let addr = SocketAddr::from(([127, 0, 0, 1], 3499));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
