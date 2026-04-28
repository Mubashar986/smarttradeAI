use axum::serve;
use server::{app, run_turn_worker, AppState};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);
    let address = format!("{host}:{port}");

    let listener = TcpListener::bind(&address)
        .await
        .unwrap_or_else(|error| panic!("failed to bind {address}: {error}"));
    let (state, turn_rx) = AppState::new();
    tokio::spawn(run_turn_worker(state.clone(), turn_rx));

    println!("smarttrade-c2-engine listening on http://{address}");

    serve(listener, app(state))
        .await
        .unwrap_or_else(|error| panic!("server exited unexpectedly: {error}"));
}
