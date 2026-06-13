use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::state::AppState;

pub(crate) async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "smarttrade-c2-engine",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub(crate) async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let ready = !state.turn_tx.is_closed();
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(serde_json::json!({
            "status": if ready { "ready" } else { "not_ready" },
            "service": "smarttrade-c2-engine",
            "worker_channel_open": ready,
        })),
    )
}
