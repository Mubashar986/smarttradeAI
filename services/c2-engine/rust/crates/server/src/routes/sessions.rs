use std::convert::Infallible;
use std::time::Duration;

use async_stream::stream;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures_util::StreamExt;
use tokio::sync::broadcast;

use crate::state::{
    AppState, ApiResult, CreateSessionResponse, ListSessionsResponse, Session,
    SessionDetailsResponse, SessionEvent, SessionId, SessionSummary, not_found,
};

pub(crate) async fn create_session(
    State(state): State<AppState>,
) -> (StatusCode, Json<CreateSessionResponse>) {
    let session_id = state.allocate_session_id();
    let session = Session::new(session_id.clone());

    state
        .sessions
        .write()
        .await
        .insert(session_id.clone(), session);
    let _ = state.turn_lock_for(&session_id).await;

    (
        StatusCode::CREATED,
        Json(CreateSessionResponse { session_id }),
    )
}

pub(crate) async fn list_sessions(State(state): State<AppState>) -> Json<ListSessionsResponse> {
    let sessions = state.sessions.read().await;
    let mut summaries = sessions
        .values()
        .map(|session| SessionSummary {
            id: session.id.clone(),
            created_at: session.created_at,
            message_count: session.conversation.messages.len(),
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.id.cmp(&right.id));

    Json(ListSessionsResponse {
        sessions: summaries,
    })
}

pub(crate) async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
) -> ApiResult<Json<SessionDetailsResponse>> {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&id)
        .ok_or_else(|| not_found(format!("session `{id}` not found")))?;

    Ok(Json(SessionDetailsResponse {
        id: session.id.clone(),
        created_at: session.created_at,
        session: session.conversation.clone(),
    }))
}

pub(crate) async fn stream_session_events(
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
) -> ApiResult<impl IntoResponse> {
    let (snapshot, mut receiver) = {
        let sessions = state.sessions.read().await;
        let session = sessions
            .get(&id)
            .ok_or_else(|| not_found(format!("session `{id}` not found")))?;
        (
            SessionEvent::Snapshot {
                session_id: session.id.clone(),
                session: session.conversation.clone(),
            },
            session.subscribe(),
        )
    };

    let stream = stream! {
        if let Ok(event) = snapshot.to_sse_event() {
            yield Ok::<Event, Infallible>(event);
        }

        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Ok(sse_event) = event.to_sse_event() {
                        yield Ok::<Event, Infallible>(sse_event);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

pub(crate) async fn stream_session_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
) -> ApiResult<impl IntoResponse> {
    let (snapshot, receiver) = {
        let sessions = state.sessions.read().await;
        let session = sessions
            .get(&id)
            .ok_or_else(|| not_found(format!("session `{id}` not found")))?;
        (
            SessionEvent::Snapshot {
                session_id: session.id.clone(),
                session: session.conversation.clone(),
            },
            session.subscribe(),
        )
    };

    Ok(ws.on_upgrade(move |socket| websocket_session_events(socket, snapshot, receiver)))
}

async fn websocket_session_events(
    mut socket: WebSocket,
    snapshot: SessionEvent,
    mut receiver: broadcast::Receiver<SessionEvent>,
) {
    if send_websocket_event(&mut socket, &snapshot).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Ok(WsMessage::Ping(payload))) => {
                        if socket.send(WsMessage::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
            outgoing = receiver.recv() => {
                match outgoing {
                    Ok(event) => {
                        if send_websocket_event(&mut socket, &event).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn send_websocket_event(
    socket: &mut WebSocket,
    event: &SessionEvent,
) -> Result<(), String> {
    let payload = serde_json::to_string(event).map_err(|error| error.to_string())?;
    socket
        .send(WsMessage::Text(payload.into()))
        .await
        .map_err(|error| error.to_string())
}
