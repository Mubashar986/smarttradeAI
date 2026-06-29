use axum::serve;
use server::{app, run_turn_worker, AppState};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize structured logging. Defaults to INFO, override with RUST_LOG env var.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);
    let address = format!("{host}:{port}");

    let listener = TcpListener::bind(&address)
        .await
        .unwrap_or_else(|error| panic!("failed to bind {address}: {error}"));
    let database_url = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
    let pool = if let Some(url) = database_url {
        tracing::info!("connecting to PostgreSQL database...");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(&url)
            .await
            .unwrap_or_else(|err| panic!("failed to connect to database {url}: {err}"));
        Some(pool)
    } else {
        tracing::warn!("DATABASE_URL is not set — falling back to local file storage mode");
        None
    };

    let (state, turn_rx) = AppState::new(pool);

    tracing::info!(
        model = %state.llm_model,
        "starting turn worker"
    );
    tokio::spawn(run_turn_worker(state.clone(), turn_rx));

    tracing::info!(address = %address, "smarttrade-c2-engine listening");

    serve(listener, app(state))
        .await
        .unwrap_or_else(|error| panic!("server exited unexpectedly: {error}"));
}
