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
    let app_env = std::env::var("APP_ENV").ok();
    let is_production = app_env.as_deref()
        .map(|s| s.eq_ignore_ascii_case("production"))
        .unwrap_or(false);

    let database_url = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());

    if is_production && database_url.is_none() {
        tracing::error!("FATAL: DATABASE_URL environment variable is missing or empty under production profile (APP_ENV=production)!");
        std::process::exit(1);
    }
    let pool = if let Some(url) = database_url {
        tracing::info!("connecting to PostgreSQL database...");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(&url)
            .await
            .unwrap_or_else(|err| panic!("failed to connect to database {url}: {err}"));
        tracing::info!("running PostgreSQL schema migrations...");
        sqlx::migrate!("../../../plugins/smarttrade-mql5/db/migrations")
            .run(&pool)
            .await
            .unwrap_or_else(|err| panic!("failed to run database migrations: {err}"));
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
