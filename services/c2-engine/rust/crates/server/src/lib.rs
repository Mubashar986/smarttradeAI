mod auth;

use std::fs;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path as FsPath, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_stream::stream;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::Row;
use tower_http::cors::{Any, CorsLayer};
use runtime::{
    classify_intent, detect_ambiguity, extract_strategy_spec, generate_strategy_code,
    run_static_analysis, AmbiguityStatus, ContentBlock, ConversationMessage, SmartTradeToolConfig,
    Session as RuntimeSession, StrategyIntent,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use crate::auth::JwtAuthConfig;

pub type SessionId = String;
pub type SessionStore = Arc<RwLock<HashMap<SessionId, Session>>>;
pub type TaskId = String;
pub type TaskStore = Arc<RwLock<HashMap<TaskId, TurnTask>>>;

const BROADCAST_CAPACITY: usize = 64;

/// A request to run one LLM turn for a given session.
#[derive(Debug, Clone)]
pub struct TurnRequest {
    pub task_id: TaskId,
    pub session_id: SessionId,
    pub user_message: String,
    pub message_type: TurnMessageType,
    pub context: TurnContext,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TurnMessageType {
    #[default]
    Intent,
    ClarificationResponse,
    ExplanationRequest,
}

impl TurnMessageType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intent => "intent",
            Self::ClarificationResponse => "clarification_response",
            Self::ExplanationRequest => "explanation_request",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskResultType {
    Clarification,
    Generation,
    Explanation,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TurnTask {
    pub id: TaskId,
    pub session_id: SessionId,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_type: Option<TaskResultType>,
    pub payload: JsonValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub message_type: TurnMessageType,
    pub context: TurnContext,
    pub created_at: u64,
    pub updated_at: u64,
}

impl TurnTask {
    fn queued(
        id: TaskId,
        session_id: SessionId,
        message_type: TurnMessageType,
        context: TurnContext,
    ) -> Self {
        let now = unix_timestamp_millis();
        Self {
            id,
            session_id,
            status: TaskStatus::Queued,
            result_type: None,
            payload: json!({ "phase": "queued" }),
            error: None,
            message_type,
            context,
            created_at: now,
            updated_at: now,
        }
    }

    fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
        self.updated_at = unix_timestamp_millis();
        self.payload = json!({ "phase": "running" });
        self.error = None;
    }

    fn complete(&mut self, result_type: TaskResultType, payload: JsonValue) {
        self.status = TaskStatus::Completed;
        self.result_type = Some(result_type);
        self.payload = payload;
        self.error = None;
        self.updated_at = unix_timestamp_millis();
    }

    fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.result_type = Some(TaskResultType::Error);
        self.payload = json!({ "error": error });
        self.error = Some(error);
        self.updated_at = unix_timestamp_millis();
    }
}

#[derive(Clone)]
pub struct AppState {
    pub sessions: SessionStore,
    pub tasks: TaskStore,
    next_session_id: Arc<AtomicU64>,
    next_task_id: Arc<AtomicU64>,
    turn_locks: Arc<RwLock<HashMap<SessionId, Arc<Mutex<()>>>>>,
    clarification_rounds: Arc<RwLock<HashMap<SessionId, u64>>>,
    turn_tx: mpsc::UnboundedSender<TurnRequest>,
}

impl AppState {
    #[must_use]
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TurnRequest>) {
        let (turn_tx, turn_rx) = mpsc::unbounded_channel();
        (
            Self {
                sessions: Arc::new(RwLock::new(HashMap::new())),
                tasks: Arc::new(RwLock::new(HashMap::new())),
                next_session_id: Arc::new(AtomicU64::new(1)),
                next_task_id: Arc::new(AtomicU64::new(1)),
                turn_locks: Arc::new(RwLock::new(HashMap::new())),
                clarification_rounds: Arc::new(RwLock::new(HashMap::new())),
                turn_tx,
            },
            turn_rx,
        )
    }

    fn allocate_session_id(&self) -> SessionId {
        let id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        format!("session-{id}")
    }

    fn allocate_task_id(&self) -> TaskId {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        format!("task-{id}")
    }

    pub async fn turn_lock_for(&self, session_id: &str) -> Arc<Mutex<()>> {
        if let Some(lock) = self.turn_locks.read().await.get(session_id).cloned() {
            return lock;
        }

        let mut locks = self.turn_locks.write().await;
        locks
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub async fn mark_task_running(&self, task_id: &str) {
        if let Some(task) = self.tasks.write().await.get_mut(task_id) {
            task.mark_running();
        }
    }

    pub async fn complete_task(
        &self,
        task_id: &str,
        result_type: TaskResultType,
        payload: JsonValue,
    ) {
        if let Some(task) = self.tasks.write().await.get_mut(task_id) {
            task.complete(result_type, payload);
        }
    }

    pub async fn fail_task(&self, task_id: &str, error: String) {
        if let Some(task) = self.tasks.write().await.get_mut(task_id) {
            task.fail(error);
        }
    }

    pub async fn next_clarification_round(&self, session_id: &str) -> u64 {
        let mut rounds = self.clarification_rounds.write().await;
        let round = rounds.entry(session_id.to_string()).or_insert(0);
        *round += 1;
        *round
    }

    pub async fn clear_clarification_rounds(&self, session_id: &str) {
        self.clarification_rounds.write().await.remove(session_id);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new().0
    }
}

#[derive(Clone)]
pub struct Session {
    pub id: SessionId,
    pub created_at: u64,
    pub conversation: RuntimeSession,
    events: broadcast::Sender<SessionEvent>,
}

impl Session {
    fn new(id: SessionId) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            id,
            created_at: unix_timestamp_millis(),
            conversation: RuntimeSession::new(),
            events,
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.events.subscribe()
    }

    /// Broadcast an event to all SSE listeners for this session.
    pub fn broadcast(&self, event: SessionEvent) {
        let _ = self.events.send(event);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    Snapshot {
        session_id: SessionId,
        session: RuntimeSession,
    },
    Message {
        session_id: SessionId,
        message: ConversationMessage,
    },
    AssistantReply {
        session_id: SessionId,
        message: ConversationMessage,
    },
    TurnComplete {
        session_id: SessionId,
        iterations: usize,
    },
    TurnError {
        session_id: SessionId,
        error: String,
    },
    Status {
        session_id: SessionId,
        task_id: TaskId,
        phase: String,
        message: String,
    },
    ClarificationQuestion {
        session_id: SessionId,
        task_id: TaskId,
        prompt: String,
        target_field: Option<String>,
        missing_fields: Vec<String>,
        round: Option<u64>,
        max_rounds: Option<u64>,
    },
    ValidationFeedback {
        session_id: SessionId,
        task_id: TaskId,
        stage: String,
        passed: bool,
        details: JsonValue,
    },
    GeneratedCode {
        session_id: SessionId,
        task_id: TaskId,
        content: String,
    },
    Error {
        session_id: SessionId,
        task_id: TaskId,
        message: String,
    },
}

impl SessionEvent {
    fn event_name(&self) -> &'static str {
        match self {
            Self::Snapshot { .. } => "snapshot",
            Self::Message { .. } => "message",
            Self::AssistantReply { .. } => "assistant_reply",
            Self::TurnComplete { .. } => "turn_complete",
            Self::TurnError { .. } => "turn_error",
            Self::Status { .. } => "status",
            Self::ClarificationQuestion { .. } => "clarification_question",
            Self::ValidationFeedback { .. } => "validation_feedback",
            Self::GeneratedCode { .. } => "generated_code",
            Self::Error { .. } => "error",
        }
    }

    fn to_sse_event(&self) -> Result<Event, serde_json::Error> {
        Ok(Event::default()
            .event(self.event_name())
            .data(serde_json::to_string(self)?))
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateSessionResponse {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSummary {
    pub id: SessionId,
    pub created_at: u64,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDetailsResponse {
    pub id: SessionId,
    pub created_at: u64,
    pub session: RuntimeSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendMessageRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmitTurnRequest {
    #[serde(default)]
    pub message_type: TurnMessageType,
    pub text: String,
    #[serde(default)]
    pub context: TurnContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmitTurnResponse {
    pub task_id: TaskId,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskStatusResponse {
    pub task_id: TaskId,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_type: Option<TaskResultType>,
    pub payload: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategySummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub session_id: String,
    pub pair: String,
    pub timeframe: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyRecord {
    pub id: String,
    pub name: String,
    pub code: String,
    pub explanation: String,
    pub status: String,
    pub session_id: String,
    pub user_id: String,
    pub pair: String,
    pub timeframe: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListStrategiesResponse {
    pub strategies: Vec<StrategySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyDetailsResponse {
    pub strategy: StrategyRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UpdateStrategyRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pair: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteStrategyResponse {
    pub strategy_id: String,
    pub status: String,
}

#[must_use]
pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Canonical frontend-facing API surface.
    let protected_v1 = Router::new()
        .route("/v1/sessions", post(create_session).get(list_sessions))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/turn", post(send_turn))
        .route("/v1/sessions/{id}/events", get(stream_session_events))
        .route("/v1/ws/{id}", get(stream_session_websocket))
        .route("/v1/tasks/{task_id}", get(get_task))
        .route("/v1/strategies", get(list_strategies))
        .route(
            "/v1/strategies/{id}",
            get(get_strategy).patch(patch_strategy).delete(delete_strategy),
        )
        .route_layer(middleware::from_fn_with_state(
            JwtAuthConfig::from_env(),
            auth::require_jwt,
        ));

    // Legacy compatibility/debug routes. Keep behavior stable until callers migrate to /v1.
    Router::new()
        .route("/health", get(health))
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/sessions", post(create_session).get(list_sessions))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/events", get(stream_session_events))
        .route("/sessions/{id}/message", post(send_message))
        .merge(protected_v1)
        .layer(cors)
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "smarttrade-c2-engine",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn ready(State(state): State<AppState>) -> impl IntoResponse {
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

async fn create_session(
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

async fn list_sessions(State(state): State<AppState>) -> Json<ListSessionsResponse> {
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

async fn get_session(
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

async fn send_message(
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
    Json(payload): Json<SendMessageRequest>,
) -> ApiResult<StatusCode> {
    enqueue_turn(
        &state,
        &id,
        SubmitTurnRequest {
            message_type: TurnMessageType::Intent,
            text: payload.message,
            context: TurnContext::default(),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn send_turn(
    State(state): State<AppState>,
    claims: Option<Extension<auth::AuthClaims>>,
    Path(id): Path<SessionId>,
    Json(mut payload): Json<SubmitTurnRequest>,
) -> ApiResult<(StatusCode, Json<SubmitTurnResponse>)> {
    if payload.context.user_id.is_none() {
        payload.context.user_id = claims
            .as_ref()
            .and_then(|claims| claims.0.principal_id().map(ToOwned::to_owned));
    }
    let task_id = enqueue_turn(&state, &id, payload).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(SubmitTurnResponse {
            task_id,
            status: TaskStatus::Queued,
        }),
    ))
}

async fn list_strategies(
    claims: Option<Extension<auth::AuthClaims>>,
) -> ApiResult<Json<ListStrategiesResponse>> {
    let user_id = resolved_user_id(claims);
    let strategies = load_strategies_for_user(&user_id)
        .await
        .map_err(internal_error)?;
    Ok(Json(ListStrategiesResponse {
        strategies: strategies.into_iter().map(StrategyRecord::summary).collect(),
    }))
}

async fn get_strategy(
    claims: Option<Extension<auth::AuthClaims>>,
    Path(id): Path<String>,
) -> ApiResult<Json<StrategyDetailsResponse>> {
    let user_id = resolved_user_id(claims);
    let strategy = load_strategy_record(&user_id, &id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("strategy `{id}` not found")))?;
    Ok(Json(StrategyDetailsResponse { strategy }))
}

async fn patch_strategy(
    claims: Option<Extension<auth::AuthClaims>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateStrategyRequest>,
) -> ApiResult<Json<StrategyDetailsResponse>> {
    if payload.name.is_none()
        && payload.code.is_none()
        && payload.explanation.is_none()
        && payload.status.is_none()
        && payload.pair.is_none()
        && payload.timeframe.is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "at least one strategy field must be provided".to_string(),
            }),
        ));
    }

    let user_id = resolved_user_id(claims);
    let strategy = update_strategy_record(&user_id, &id, &payload)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("strategy `{id}` not found")))?;
    Ok(Json(StrategyDetailsResponse { strategy }))
}

async fn delete_strategy(
    claims: Option<Extension<auth::AuthClaims>>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteStrategyResponse>> {
    let user_id = resolved_user_id(claims);
    let deleted = soft_delete_strategy_record(&user_id, &id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("strategy `{id}` not found")))?;
    Ok(Json(deleted))
}

async fn enqueue_turn(
    state: &AppState,
    session_id: &str,
    payload: SubmitTurnRequest,
) -> ApiResult<TaskId> {
    let task_id = state.allocate_task_id();
    let mut context = payload.context;
    context.task_id = Some(task_id.clone());
    let message = ConversationMessage::user_text(payload.text.clone());
    let broadcaster = {
        let mut sessions = state.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| not_found(format!("session `{session_id}` not found")))?;
        session.conversation.messages.push(message.clone());
        session.events.clone()
    };

    state.tasks.write().await.insert(
        task_id.clone(),
        TurnTask::queued(
            task_id.clone(),
            session_id.to_string(),
            payload.message_type,
            context.clone(),
        ),
    );

    let _ = broadcaster.send(SessionEvent::Message {
        session_id: session_id.to_string(),
        message,
    });
    let _ = broadcaster.send(SessionEvent::Status {
        session_id: session_id.to_string(),
        task_id: task_id.clone(),
        phase: "queued".to_string(),
        message: "turn accepted".to_string(),
    });

    if state
        .turn_tx
        .send(TurnRequest {
            task_id: task_id.clone(),
            session_id: session_id.to_string(),
            user_message: payload.text,
            message_type: payload.message_type,
            context,
        })
        .is_err()
    {
        state.tasks.write().await.remove(&task_id);
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "turn worker unavailable".to_string(),
            }),
        ));
    }

    Ok(task_id)
}

async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<TaskId>,
) -> ApiResult<Json<TaskStatusResponse>> {
    let tasks = state.tasks.read().await;
    let task = tasks
        .get(&task_id)
        .ok_or_else(|| not_found(format!("task `{task_id}` not found")))?;
    Ok(Json(TaskStatusResponse {
        task_id: task.id.clone(),
        status: task.status,
        result_type: task.result_type,
        payload: task.payload.clone(),
    }))
}

async fn stream_session_events(
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

async fn stream_session_websocket(
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

pub async fn run_turn_worker(state: AppState, mut turn_rx: mpsc::UnboundedReceiver<TurnRequest>) {
    while let Some(request) = turn_rx.recv().await {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = process_turn(state.clone(), request.clone()).await {
                state.fail_task(&request.task_id, error.clone()).await;
                broadcast_event(
                    &state,
                    &request.session_id,
                    SessionEvent::Error {
                        session_id: request.session_id.clone(),
                        task_id: request.task_id.clone(),
                        message: error.clone(),
                    },
                )
                .await;
                broadcast_event(
                    &state,
                    &request.session_id,
                    SessionEvent::TurnError {
                        session_id: request.session_id.clone(),
                        error,
                    },
                )
                .await;
            }
        });
    }
}

async fn process_turn(state: AppState, request: TurnRequest) -> Result<(), String> {
    let lock = state.turn_lock_for(&request.session_id).await;
    let _guard = lock.lock().await;

    state.mark_task_running(&request.task_id).await;
    broadcast_status(
        &state,
        &request.session_id,
        &request.task_id,
        "running",
        "processing turn",
    )
    .await;

    let combined_text = combined_user_text(&state, &request.session_id).await?;
    let classification = classify_intent(&request.user_message);
    broadcast_status(
        &state,
        &request.session_id,
        &request.task_id,
        "classified",
        &format!(
            "intent={} confidence={:.2}",
            classification.intent.as_str(),
            classification.confidence
        ),
    )
    .await;

    match classification.intent {
        StrategyIntent::StrategyCreation
        | StrategyIntent::StrategyRefinement
        | StrategyIntent::ClarificationResponse => {
            let spec = extract_strategy_spec(&combined_text);
            let round = state.next_clarification_round(&request.session_id).await;
            let ambiguity = detect_ambiguity(&spec, round);
            match ambiguity.status {
                AmbiguityStatus::Incomplete => {
                    let prompt = ambiguity
                        .next_question
                        .clone()
                        .unwrap_or_else(|| "Please provide the missing strategy detail.".to_string());
                    let reply = ConversationMessage::assistant(vec![ContentBlock::Text {
                        text: format!("I need one more detail before I can continue: {prompt}"),
                    }]);
                    append_assistant_reply(&state, &request.session_id, reply).await?;
                    broadcast_event(
                        &state,
                        &request.session_id,
                        SessionEvent::ClarificationQuestion {
                            session_id: request.session_id.clone(),
                            task_id: request.task_id.clone(),
                            prompt,
                            target_field: ambiguity.missing_fields.first().cloned(),
                            missing_fields: ambiguity.missing_fields.clone(),
                            round: Some(ambiguity.round),
                            max_rounds: Some(ambiguity.max_rounds),
                        },
                    )
                    .await;
                    state
                        .complete_task(
                            &request.task_id,
                            TaskResultType::Clarification,
                            json!({
                                "status": ambiguity.status,
                                "round": ambiguity.round,
                                "max_rounds": ambiguity.max_rounds,
                                "missing_fields": ambiguity.missing_fields,
                                "missing_count": ambiguity.missing_count,
                                "provided_fields": ambiguity.provided_fields,
                                "next_question": ambiguity.next_question,
                                "classification": {
                                    "intent": classification.intent,
                                    "confidence": classification.confidence,
                                    "all_scores": classification.all_scores,
                                },
                                "spec": ambiguity.spec,
                            }),
                        )
                        .await;
                    broadcast_status(
                        &state,
                        &request.session_id,
                        &request.task_id,
                        "waiting_for_clarification",
                        "more strategy detail is required",
                    )
                    .await;
                }
                AmbiguityStatus::Complete => {
                    state.clear_clarification_rounds(&request.session_id).await;
                    let generated = generate_strategy_code(&combined_text, &spec)
                        .map_err(|error| error.to_string())?;
                    let analysis = run_static_analysis(&generated.code, 1);
                    let analysis_details = serde_json::to_value(&analysis).unwrap_or_else(|_| {
                        json!({
                            "passed": false,
                            "status": "SERIALIZATION_ERROR",
                            "message": "failed to serialize static analysis result",
                        })
                    });

                    broadcast_event(
                        &state,
                        &request.session_id,
                        SessionEvent::GeneratedCode {
                            session_id: request.session_id.clone(),
                            task_id: request.task_id.clone(),
                            content: generated.code.clone(),
                        },
                    )
                    .await;
                    broadcast_event(
                        &state,
                        &request.session_id,
                        SessionEvent::ValidationFeedback {
                            session_id: request.session_id.clone(),
                            task_id: request.task_id.clone(),
                            stage: "static_analysis".to_string(),
                            passed: analysis.passed,
                            details: analysis_details.clone(),
                        },
                    )
                    .await;

                    let reply = ConversationMessage::assistant(vec![ContentBlock::Text {
                        text: if analysis.passed {
                            format!(
                                "I generated an MQL5 draft using the {} skeleton and static analysis passed. Compilation and persistence are still the next integration steps.",
                                generated.skeleton_type
                            )
                        } else {
                            format!(
                                "I generated an MQL5 draft using the {} skeleton, but static analysis found issues that still need an automated correction loop.",
                                generated.skeleton_type
                            )
                        },
                    }]);
                    append_assistant_reply(&state, &request.session_id, reply).await?;
                    broadcast_event(
                        &state,
                        &request.session_id,
                        SessionEvent::ValidationFeedback {
                            session_id: request.session_id.clone(),
                            task_id: request.task_id.clone(),
                            stage: "spec_capture".to_string(),
                            passed: true,
                            details: json!({
                                "classification": {
                                    "intent": classification.intent,
                                    "confidence": classification.confidence,
                                    "all_scores": classification.all_scores,
                                },
                                "spec": ambiguity.spec,
                            }),
                        },
                    )
                    .await;
                    state
                        .complete_task(
                            &request.task_id,
                            TaskResultType::Generation,
                            json!({
                                "status": ambiguity.status,
                                "round": ambiguity.round,
                                "classification": {
                                    "intent": classification.intent,
                                    "confidence": classification.confidence,
                                    "all_scores": classification.all_scores,
                                },
                                "spec": ambiguity.spec,
                                "generation": {
                                    "strategy_name": generated.strategy_name,
                                    "skeleton_type": generated.skeleton_type,
                                    "code": generated.code,
                                    "explanation": generated.explanation,
                                    "lines": generated.lines,
                                },
                                "analysis": analysis,
                                "ready_for_compile": analysis.passed,
                            }),
                        )
                        .await;
                    broadcast_status(
                        &state,
                        &request.session_id,
                        &request.task_id,
                        "generation_complete",
                        "generated code and static validation finished",
                    )
                    .await;
                }
                AmbiguityStatus::DraftSaved => {
                    let reply = ConversationMessage::assistant(vec![ContentBlock::Text {
                        text: ambiguity.message.clone(),
                    }]);
                    append_assistant_reply(&state, &request.session_id, reply).await?;
                    state
                        .complete_task(
                            &request.task_id,
                            TaskResultType::Clarification,
                            json!({
                                "status": ambiguity.status,
                                "round": ambiguity.round,
                                "max_rounds": ambiguity.max_rounds,
                                "provided_fields": ambiguity.provided_fields,
                                "spec": ambiguity.spec,
                                "message": ambiguity.message,
                            }),
                        )
                        .await;
                    broadcast_status(
                        &state,
                        &request.session_id,
                        &request.task_id,
                        "draft_saved",
                        "maximum clarification rounds exceeded",
                    )
                    .await;
                }
            }
        }
        StrategyIntent::ExplanationRequest | StrategyIntent::General => {
            let response_text = if classification.intent == StrategyIntent::ExplanationRequest {
                "I can help explain the SmartTrade strategy workflow, but the native explanation path is still being wired into the runtime."
            } else {
                "Please describe the trading strategy you want to automate, including pair, timeframe, entry, exit, and stop-loss."
            };
            let reply = ConversationMessage::assistant(vec![ContentBlock::Text {
                text: response_text.to_string(),
            }]);
            append_assistant_reply(&state, &request.session_id, reply).await?;
            state
                .complete_task(
                    &request.task_id,
                    TaskResultType::Explanation,
                    json!({
                        "status": "responded",
                        "classification": {
                            "intent": classification.intent,
                            "confidence": classification.confidence,
                            "all_scores": classification.all_scores,
                        },
                        "message": response_text,
                    }),
                )
                .await;
            broadcast_status(
                &state,
                &request.session_id,
                &request.task_id,
                "responded",
                "assistant reply generated",
            )
            .await;
        }
    }

    broadcast_event(
        &state,
        &request.session_id,
        SessionEvent::TurnComplete {
            session_id: request.session_id.clone(),
            iterations: 1,
        },
    )
    .await;
    Ok(())
}

async fn combined_user_text(state: &AppState, session_id: &str) -> Result<String, String> {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(session_id)
        .ok_or_else(|| format!("session `{session_id}` not found"))?;
    Ok(session
        .conversation
        .messages
        .iter()
        .filter(|message| matches!(message.role, runtime::MessageRole::User))
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            ContentBlock::ToolUse { .. } | ContentBlock::ToolResult { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

async fn append_assistant_reply(
    state: &AppState,
    session_id: &str,
    message: ConversationMessage,
) -> Result<(), String> {
    let sender = {
        let mut sessions = state.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("session `{session_id}` not found"))?;
        session.conversation.messages.push(message.clone());
        session.events.clone()
    };
    let _ = sender.send(SessionEvent::AssistantReply {
        session_id: session_id.to_string(),
        message,
    });
    Ok(())
}

async fn broadcast_status(
    state: &AppState,
    session_id: &str,
    task_id: &str,
    phase: &str,
    message: &str,
) {
    broadcast_event(
        state,
        session_id,
        SessionEvent::Status {
            session_id: session_id.to_string(),
            task_id: task_id.to_string(),
            phase: phase.to_string(),
            message: message.to_string(),
        },
    )
    .await;
}

async fn broadcast_event(state: &AppState, session_id: &str, event: SessionEvent) {
    let sender = {
        let sessions = state.sessions.read().await;
        sessions.get(session_id).map(|session| session.events.clone())
    };
    if let Some(sender) = sender {
        let _ = sender.send(event);
    }
}

impl StrategyRecord {
    fn summary(self) -> StrategySummary {
        StrategySummary {
            id: self.id,
            name: self.name,
            status: self.status,
            session_id: self.session_id,
            pair: self.pair,
            timeframe: self.timeframe,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

async fn load_strategies_for_user(user_id: &str) -> Result<Vec<StrategyRecord>, String> {
    let storage = SmartTradeToolConfig::from_env();
    match storage.database_url {
        Some(database_url) => load_db_strategies(&database_url, user_id).await,
        None => load_local_strategies(&storage.strategies_dir, user_id),
    }
}

async fn load_strategy_record(
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<StrategyRecord>, String> {
    let storage = SmartTradeToolConfig::from_env();
    match storage.database_url {
        Some(database_url) => load_db_strategy(&database_url, user_id, strategy_id).await,
        None => load_local_strategy(&storage.strategies_dir, user_id, strategy_id),
    }
}

async fn update_strategy_record(
    user_id: &str,
    strategy_id: &str,
    update: &UpdateStrategyRequest,
) -> Result<Option<StrategyRecord>, String> {
    let storage = SmartTradeToolConfig::from_env();
    match storage.database_url {
        Some(database_url) => {
            update_db_strategy(&database_url, user_id, strategy_id, update).await
        }
        None => update_local_strategy(&storage.strategies_dir, user_id, strategy_id, update),
    }
}

async fn soft_delete_strategy_record(
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<DeleteStrategyResponse>, String> {
    let storage = SmartTradeToolConfig::from_env();
    match storage.database_url {
        Some(database_url) => soft_delete_db_strategy(&database_url, user_id, strategy_id).await,
        None => soft_delete_local_strategy(&storage.strategies_dir, user_id, strategy_id),
    }
}

async fn load_db_strategies(
    database_url: &str,
    user_id: &str,
) -> Result<Vec<StrategyRecord>, String> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())?;
    let rows = sqlx::query(
        r#"
        SELECT
            id::text AS id,
            name,
            code,
            explanation,
            status,
            session_id,
            user_id,
            pair,
            timeframe,
            created_at::text AS created_at,
            updated_at::text AS updated_at
        FROM strategies
        WHERE user_id = $1 AND status <> 'DELETED'
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .map_err(|error| error.to_string())?;

    rows.into_iter().map(strategy_record_from_row).collect()
}

async fn load_db_strategy(
    database_url: &str,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<StrategyRecord>, String> {
    let Ok(strategy_id) = strategy_id.parse::<i64>() else {
        return Ok(None);
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())?;
    let row = sqlx::query(
        r#"
        SELECT
            id::text AS id,
            name,
            code,
            explanation,
            status,
            session_id,
            user_id,
            pair,
            timeframe,
            created_at::text AS created_at,
            updated_at::text AS updated_at
        FROM strategies
        WHERE user_id = $1 AND id = $2 AND status <> 'DELETED'
        "#,
    )
    .bind(user_id)
    .bind(strategy_id)
    .fetch_optional(&pool)
    .await
    .map_err(|error| error.to_string())?;

    row.map(strategy_record_from_row).transpose()
}

async fn update_db_strategy(
    database_url: &str,
    user_id: &str,
    strategy_id: &str,
    update: &UpdateStrategyRequest,
) -> Result<Option<StrategyRecord>, String> {
    let Ok(strategy_id) = strategy_id.parse::<i64>() else {
        return Ok(None);
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())?;
    let row = sqlx::query(
        r#"
        UPDATE strategies
        SET
            name = COALESCE($3, name),
            code = COALESCE($4, code),
            explanation = COALESCE($5, explanation),
            status = COALESCE($6, status),
            pair = COALESCE($7, pair),
            timeframe = COALESCE($8, timeframe),
            updated_at = NOW()
        WHERE user_id = $1 AND id = $2 AND status <> 'DELETED'
        RETURNING
            id::text AS id,
            name,
            code,
            explanation,
            status,
            session_id,
            user_id,
            pair,
            timeframe,
            created_at::text AS created_at,
            updated_at::text AS updated_at
        "#,
    )
    .bind(user_id)
    .bind(strategy_id)
    .bind(update.name.clone())
    .bind(update.code.clone())
    .bind(update.explanation.clone())
    .bind(update.status.clone())
    .bind(update.pair.clone())
    .bind(update.timeframe.clone())
    .fetch_optional(&pool)
    .await
    .map_err(|error| error.to_string())?;

    row.map(strategy_record_from_row).transpose()
}

async fn soft_delete_db_strategy(
    database_url: &str,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<DeleteStrategyResponse>, String> {
    let Ok(strategy_id_num) = strategy_id.parse::<i64>() else {
        return Ok(None);
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())?;
    let row = sqlx::query(
        r#"
        UPDATE strategies
        SET status = 'DELETED', updated_at = NOW()
        WHERE user_id = $1 AND id = $2 AND status <> 'DELETED'
        RETURNING id::text AS id, status
        "#,
    )
    .bind(user_id)
    .bind(strategy_id_num)
    .fetch_optional(&pool)
    .await
    .map_err(|error| error.to_string())?;

    Ok(row.map(|row| DeleteStrategyResponse {
        strategy_id: row.try_get("id").unwrap_or_default(),
        status: row
            .try_get::<String, _>("status")
            .unwrap_or_else(|_| "DELETED".to_string()),
    }))
}

fn strategy_record_from_row(row: PgRow) -> Result<StrategyRecord, String> {
    Ok(StrategyRecord {
        id: row.try_get("id").map_err(|error| error.to_string())?,
        name: row.try_get("name").map_err(|error| error.to_string())?,
        code: row.try_get("code").map_err(|error| error.to_string())?,
        explanation: row
            .try_get("explanation")
            .map_err(|error| error.to_string())?,
        status: row.try_get("status").map_err(|error| error.to_string())?,
        session_id: row
            .try_get("session_id")
            .map_err(|error| error.to_string())?,
        user_id: row.try_get("user_id").map_err(|error| error.to_string())?,
        pair: row.try_get("pair").map_err(|error| error.to_string())?,
        timeframe: row
            .try_get("timeframe")
            .map_err(|error| error.to_string())?,
        created_at: row
            .try_get("created_at")
            .map_err(|error| error.to_string())?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|error| error.to_string())?,
    })
}

fn load_local_strategies(
    strategies_dir: &FsPath,
    user_id: &str,
) -> Result<Vec<StrategyRecord>, String> {
    let mut strategies = local_strategy_paths(strategies_dir)?
        .into_iter()
        .filter_map(|path| read_local_strategy_record(&path).ok())
        .filter(|strategy| strategy.user_id == user_id && strategy.status != "DELETED")
        .collect::<Vec<_>>();
    strategies.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(strategies)
}

fn load_local_strategy(
    strategies_dir: &FsPath,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<StrategyRecord>, String> {
    Ok(load_local_strategies(strategies_dir, user_id)?
        .into_iter()
        .find(|strategy| strategy.id == strategy_id))
}

fn update_local_strategy(
    strategies_dir: &FsPath,
    user_id: &str,
    strategy_id: &str,
    update: &UpdateStrategyRequest,
) -> Result<Option<StrategyRecord>, String> {
    let Some(metadata_path) = find_local_strategy_metadata_path(strategies_dir, user_id, strategy_id)?
    else {
        return Ok(None);
    };

    let mut metadata = read_local_strategy_metadata(&metadata_path)?;
    if let Some(name) = &update.name {
        metadata.insert("strategy_name".to_string(), JsonValue::String(name.clone()));
    }
    if let Some(explanation) = &update.explanation {
        metadata.insert("explanation".to_string(), JsonValue::String(explanation.clone()));
    }
    if let Some(status) = &update.status {
        metadata.insert("status".to_string(), JsonValue::String(status.clone()));
    }
    if let Some(pair) = &update.pair {
        metadata.insert("pair".to_string(), JsonValue::String(pair.clone()));
    }
    if let Some(timeframe) = &update.timeframe {
        metadata.insert("timeframe".to_string(), JsonValue::String(timeframe.clone()));
    }
    metadata.insert(
        "updated_at".to_string(),
        JsonValue::String(current_iso8601_like_timestamp()),
    );

    if let Some(code) = &update.code {
        fs::write(metadata_path.with_extension("mq5"), code).map_err(|error| error.to_string())?;
    }

    write_local_strategy_metadata(&metadata_path, &metadata)?;
    read_local_strategy_record(&metadata_path).map(Some)
}

fn soft_delete_local_strategy(
    strategies_dir: &FsPath,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<DeleteStrategyResponse>, String> {
    let Some(metadata_path) = find_local_strategy_metadata_path(strategies_dir, user_id, strategy_id)?
    else {
        return Ok(None);
    };
    let mut metadata = read_local_strategy_metadata(&metadata_path)?;
    metadata.insert("status".to_string(), JsonValue::String("DELETED".to_string()));
    metadata.insert(
        "updated_at".to_string(),
        JsonValue::String(current_iso8601_like_timestamp()),
    );
    write_local_strategy_metadata(&metadata_path, &metadata)?;
    Ok(Some(DeleteStrategyResponse {
        strategy_id: strategy_id.to_string(),
        status: "DELETED".to_string(),
    }))
}

fn local_strategy_paths(strategies_dir: &FsPath) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(strategies_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn find_local_strategy_metadata_path(
    strategies_dir: &FsPath,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<PathBuf>, String> {
    for path in local_strategy_paths(strategies_dir)? {
        let metadata = read_local_strategy_metadata(&path)?;
        let record_user_id = metadata
            .get("user_id")
            .and_then(JsonValue::as_str)
            .unwrap_or_default();
        if record_user_id != user_id {
            continue;
        }
        let record_id = metadata
            .get("strategy_id")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default()
                    .to_string()
            });
        if record_id == strategy_id {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn read_local_strategy_record(metadata_path: &FsPath) -> Result<StrategyRecord, String> {
    let metadata = read_local_strategy_metadata(metadata_path)?;
    let code_path = metadata_path.with_extension("mq5");
    let code = fs::read_to_string(&code_path).unwrap_or_default();
    let id = metadata
        .get("strategy_id")
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            metadata_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_string()
        });
    Ok(StrategyRecord {
        id,
        name: metadata
            .get("strategy_name")
            .and_then(JsonValue::as_str)
            .or_else(|| metadata.get("name").and_then(JsonValue::as_str))
            .unwrap_or("Unnamed")
            .to_string(),
        code,
        explanation: metadata
            .get("explanation")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        status: metadata
            .get("status")
            .and_then(JsonValue::as_str)
            .unwrap_or("DRAFT")
            .to_string(),
        session_id: metadata
            .get("session_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        user_id: metadata
            .get("user_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        pair: metadata
            .get("pair")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        timeframe: metadata
            .get("timeframe")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        created_at: metadata
            .get("created_at")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        updated_at: metadata
            .get("updated_at")
            .and_then(JsonValue::as_str)
            .or_else(|| metadata.get("created_at").and_then(JsonValue::as_str))
            .unwrap_or("")
            .to_string(),
    })
}

fn read_local_strategy_metadata(metadata_path: &FsPath) -> Result<serde_json::Map<String, JsonValue>, String> {
    let value = serde_json::from_str::<JsonValue>(
        &fs::read_to_string(metadata_path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| "local strategy metadata must be a JSON object".to_string())
}

fn write_local_strategy_metadata(
    metadata_path: &FsPath,
    metadata: &serde_json::Map<String, JsonValue>,
) -> Result<(), String> {
    fs::write(
        metadata_path,
        serde_json::to_string_pretty(&JsonValue::Object(metadata.clone()))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn current_iso8601_like_timestamp() -> String {
    format!("unix:{}", unix_timestamp_millis())
}

fn resolved_user_id(claims: Option<Extension<auth::AuthClaims>>) -> String {
    claims
        .as_ref()
        .and_then(|claims| claims.0.principal_id())
        .unwrap_or("local-dev-user")
        .to_string()
}

fn unix_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_millis() as u64
}

fn internal_error(message: String) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: message }),
    )
}

fn not_found(message: String) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: message }),
    )
}

#[cfg(test)]
mod tests {
    use super::auth::AuthClaims;
    use super::{
        app, run_turn_worker, AppState, CreateSessionResponse, DeleteStrategyResponse,
        ListSessionsResponse, ListStrategiesResponse, SessionDetailsResponse,
        StrategyDetailsResponse, SubmitTurnRequest, SubmitTurnResponse, TaskResultType,
        TaskStatus, TaskStatusResponse, TurnContext, TurnMessageType, UpdateStrategyRequest,
    };
    use jsonwebtoken::{encode, EncodingKey, Header};
    use reqwest::Client;
    use std::fs;
    use std::net::SocketAddr;
    use std::sync::{Mutex as StdMutex, OnceLock};
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio::time::{sleep, timeout};

    struct TestServer {
        address: SocketAddr,
        handle: JoinHandle<()>,
        worker_handle: Option<JoinHandle<()>>,
    }

    impl TestServer {
        async fn spawn() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("test listener should bind");
            let address = listener
                .local_addr()
                .expect("listener should report local address");
            let handle = tokio::spawn(async move {
                axum::serve(listener, app(AppState::default()))
                    .await
                    .expect("server should run");
            });

            Self {
                address,
                handle,
                worker_handle: None,
            }
        }

        async fn spawn_with_worker() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("test listener should bind");
            let address = listener
                .local_addr()
                .expect("listener should report local address");
            let (state, turn_rx) = AppState::new();
            let worker_state = state.clone();
            let worker_handle = tokio::spawn(async move {
                run_turn_worker(worker_state, turn_rx).await;
            });
            let handle = tokio::spawn(async move {
                axum::serve(listener, app(state))
                    .await
                    .expect("server should run");
            });

            Self {
                address,
                handle,
                worker_handle: Some(worker_handle),
            }
        }

        fn url(&self, path: &str) -> String {
            format!("http://{}{}", self.address, path)
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.handle.abort();
            if let Some(worker_handle) = &self.worker_handle {
                worker_handle.abort();
            }
        }
    }

    async fn create_session(client: &Client, server: &TestServer) -> CreateSessionResponse {
        client
            .post(server.url("/v1/sessions"))
            .send()
            .await
            .expect("create request should succeed")
            .error_for_status()
            .expect("create request should return success")
            .json::<CreateSessionResponse>()
            .await
            .expect("create response should parse")
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| StdMutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn bearer_token(secret: &str, user_id: &str) -> String {
        let claims = AuthClaims {
            sub: Some(user_id.to_string()),
            user_id: Some(user_id.to_string()),
            exp: 4_102_444_800,
            iat: Some(1_700_000_000),
            iss: None,
            aud: None,
        };
        format!(
            "Bearer {}",
            encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes())
            )
            .expect("jwt should encode")
        )
    }

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("{prefix}-{}", super::unix_timestamp_millis()))
    }

    fn seed_local_strategy(
        strategies_dir: &std::path::Path,
        strategy_id: &str,
        user_id: &str,
        status: &str,
    ) {
        fs::create_dir_all(strategies_dir).expect("strategies dir should exist");
        let stem = format!("{}_seed", strategy_id);
        fs::write(
            strategies_dir.join(format!("{stem}.mq5")),
            "void OnTick() {}",
        )
        .expect("code file should write");
        fs::write(
            strategies_dir.join(format!("{stem}.json")),
            serde_json::to_string_pretty(&serde_json::json!({
                "strategy_id": strategy_id,
                "strategy_name": format!("Strategy {strategy_id}"),
                "status": status,
                "session_id": "session-1",
                "user_id": user_id,
                "pair": "EURUSD",
                "timeframe": "H1",
                "explanation": "seeded",
                "created_at": "unix:1",
                "updated_at": "unix:2"
            }))
            .expect("seed metadata should serialize"),
        )
        .expect("metadata should write");
    }

    async fn next_sse_frame(response: &mut reqwest::Response, buffer: &mut String) -> String {
        loop {
            if let Some(index) = buffer.find("\n\n") {
                let frame = buffer[..index].to_string();
                let remainder = buffer[index + 2..].to_string();
                *buffer = remainder;
                return frame;
            }

            let next_chunk = timeout(Duration::from_secs(5), response.chunk())
                .await
                .expect("SSE stream should yield within timeout")
                .expect("SSE stream should remain readable")
                .expect("SSE stream should stay open");
            buffer.push_str(&String::from_utf8_lossy(&next_chunk));
        }
    }

    #[tokio::test]
    async fn creates_lists_and_gets_v1_sessions() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        // given
        let created = create_session(&client, &server).await;

        // when
        let sessions = client
            .get(server.url("/v1/sessions"))
            .send()
            .await
            .expect("list request should succeed")
            .error_for_status()
            .expect("list request should return success")
            .json::<ListSessionsResponse>()
            .await
            .expect("list response should parse");
        let details = client
            .get(server.url(&format!("/v1/sessions/{}", created.session_id)))
            .send()
            .await
            .expect("details request should succeed")
            .error_for_status()
            .expect("details request should return success")
            .json::<SessionDetailsResponse>()
            .await
            .expect("details response should parse");

        // then
        assert_eq!(created.session_id, "session-1");
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].id, created.session_id);
        assert_eq!(sessions.sessions[0].message_count, 0);
        assert_eq!(details.id, "session-1");
        assert!(details.session.messages.is_empty());
    }

    #[tokio::test]
    async fn legacy_session_routes_remain_available_for_compatibility() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;

        let sessions = client
            .get(server.url("/sessions"))
            .send()
            .await
            .expect("legacy list request should succeed")
            .error_for_status()
            .expect("legacy list request should return success")
            .json::<ListSessionsResponse>()
            .await
            .expect("legacy list response should parse");
        let details = client
            .get(server.url(&format!("/sessions/{}", created.session_id)))
            .send()
            .await
            .expect("legacy details request should succeed")
            .error_for_status()
            .expect("legacy details request should return success")
            .json::<SessionDetailsResponse>()
            .await
            .expect("legacy details response should parse");

        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].id, created.session_id);
        assert_eq!(details.id, created.session_id);
    }

    #[tokio::test]
    async fn accepts_v1_turn_and_exposes_task_status() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;

        let accepted = client
            .post(server.url(&format!("/v1/sessions/{}/turn", created.session_id)))
            .json(&SubmitTurnRequest {
                message_type: TurnMessageType::Intent,
                text: "buy EURUSD when 50 SMA crosses above 200 SMA".to_string(),
                context: TurnContext::default(),
            })
            .send()
            .await
            .expect("turn request should succeed")
            .error_for_status()
            .expect("turn request should return success")
            .json::<SubmitTurnResponse>()
            .await
            .expect("turn response should parse");

        let task = client
            .get(server.url(&format!("/v1/tasks/{}", accepted.task_id)))
            .send()
            .await
            .expect("task request should succeed")
            .error_for_status()
            .expect("task request should return success")
            .json::<TaskStatusResponse>()
            .await
            .expect("task response should parse");

        assert_eq!(accepted.task_id, "task-1");
        assert_eq!(accepted.status, TaskStatus::Queued);
        assert_eq!(task.task_id, accepted.task_id);
        assert_eq!(task.status, TaskStatus::Queued);
        assert!(task.result_type.is_none());
        assert_eq!(task.payload["phase"], "queued");
    }

    #[tokio::test]
    async fn rejects_protected_v1_routes_without_bearer_token_when_jwt_is_enabled() {
        let _guard = env_lock();
        std::env::set_var("C2_JWT_SECRET", "test-secret");

        let server = TestServer::spawn().await;
        let client = Client::new();

        let response = client
            .post(server.url("/v1/sessions"))
            .send()
            .await
            .expect("request should complete");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

        std::env::remove_var("C2_JWT_SECRET");
    }

    #[tokio::test]
    async fn allows_protected_v1_routes_with_valid_bearer_token_when_jwt_is_enabled() {
        let _guard = env_lock();
        let secret = "test-secret";
        std::env::set_var("C2_JWT_SECRET", secret);

        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = client
            .post(server.url("/v1/sessions"))
            .header("Authorization", bearer_token(secret, "user-1"))
            .send()
            .await
            .expect("request should complete")
            .error_for_status()
            .expect("request should be authorized")
            .json::<CreateSessionResponse>()
            .await
            .expect("response should parse");

        assert_eq!(created.session_id, "session-1");

        std::env::remove_var("C2_JWT_SECRET");
    }

    #[tokio::test]
    async fn lists_and_gets_local_strategies_for_resolved_user() {
        let _guard = env_lock();
        std::env::remove_var("C2_JWT_SECRET");
        let strategies_dir = temp_dir("server-local-strategies");
        std::env::set_var("STRATEGIES_DIR", &strategies_dir);

        seed_local_strategy(&strategies_dir, "local-one", "local-dev-user", "GENERATED");
        seed_local_strategy(&strategies_dir, "local-two", "other-user", "GENERATED");

        let server = TestServer::spawn().await;
        let client = Client::new();

        let list = client
            .get(server.url("/v1/strategies"))
            .send()
            .await
            .expect("list request should succeed")
            .error_for_status()
            .expect("list request should return success")
            .json::<ListStrategiesResponse>()
            .await
            .expect("list response should parse");

        assert_eq!(list.strategies.len(), 1);
        assert_eq!(list.strategies[0].id, "local-one");

        let details = client
            .get(server.url("/v1/strategies/local-one"))
            .send()
            .await
            .expect("details request should succeed")
            .error_for_status()
            .expect("details request should return success")
            .json::<StrategyDetailsResponse>()
            .await
            .expect("details response should parse");

        assert_eq!(details.strategy.id, "local-one");
        assert_eq!(details.strategy.user_id, "local-dev-user");

        fs::remove_dir_all(&strategies_dir).expect("cleanup temp dir");
        std::env::remove_var("STRATEGIES_DIR");
    }

    #[tokio::test]
    async fn patches_and_soft_deletes_local_strategy() {
        let _guard = env_lock();
        std::env::remove_var("C2_JWT_SECRET");
        let strategies_dir = temp_dir("server-local-strategy-update");
        std::env::set_var("STRATEGIES_DIR", &strategies_dir);

        seed_local_strategy(&strategies_dir, "local-edit", "local-dev-user", "DRAFT");

        let server = TestServer::spawn().await;
        let client = Client::new();

        let patched = client
            .patch(server.url("/v1/strategies/local-edit"))
            .json(&UpdateStrategyRequest {
                status: Some("APPROVED".to_string()),
                explanation: Some("updated explanation".to_string()),
                ..UpdateStrategyRequest::default()
            })
            .send()
            .await
            .expect("patch request should succeed")
            .error_for_status()
            .expect("patch request should return success")
            .json::<StrategyDetailsResponse>()
            .await
            .expect("patch response should parse");

        assert_eq!(patched.strategy.status, "APPROVED");
        assert_eq!(patched.strategy.explanation, "updated explanation");

        let deleted = client
            .delete(server.url("/v1/strategies/local-edit"))
            .send()
            .await
            .expect("delete request should succeed")
            .error_for_status()
            .expect("delete request should return success")
            .json::<DeleteStrategyResponse>()
            .await
            .expect("delete response should parse");

        assert_eq!(deleted.strategy_id, "local-edit");
        assert_eq!(deleted.status, "DELETED");

        let missing = client
            .get(server.url("/v1/strategies/local-edit"))
            .send()
            .await
            .expect("follow-up request should succeed");
        assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);

        fs::remove_dir_all(&strategies_dir).expect("cleanup temp dir");
        std::env::remove_var("STRATEGIES_DIR");
    }

    #[tokio::test]
    async fn streams_message_events_and_persists_message_flow() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        // given
        let created = create_session(&client, &server).await;
        let mut response = client
            .get(server.url(&format!("/sessions/{}/events", created.session_id)))
            .send()
            .await
            .expect("events request should succeed")
            .error_for_status()
            .expect("events request should return success");
        let mut buffer = String::new();
        let snapshot_frame = next_sse_frame(&mut response, &mut buffer).await;

        // when
        let send_status = client
            .post(server.url(&format!("/sessions/{}/message", created.session_id)))
            .json(&super::SendMessageRequest {
                message: "hello from test".to_string(),
            })
            .send()
            .await
            .expect("message request should succeed")
            .status();
        let message_frame = next_sse_frame(&mut response, &mut buffer).await;
        let details = client
            .get(server.url(&format!("/sessions/{}", created.session_id)))
            .send()
            .await
            .expect("details request should succeed")
            .error_for_status()
            .expect("details request should return success")
            .json::<SessionDetailsResponse>()
            .await
            .expect("details response should parse");

        // then
        assert_eq!(send_status, reqwest::StatusCode::NO_CONTENT);
        assert!(snapshot_frame.contains("event: snapshot"));
        assert!(snapshot_frame.contains("\"session_id\":\"session-1\""));
        assert!(message_frame.contains("event: message"));
        assert!(message_frame.contains("hello from test"));
        assert_eq!(details.session.messages.len(), 1);
        assert_eq!(
            details.session.messages[0],
            runtime::ConversationMessage::user_text("hello from test")
        );
    }

    #[tokio::test]
    async fn exposes_websocket_route_for_session_streaming() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;
        let response = client
            .get(server.url(&format!("/v1/ws/{}", created.session_id)))
            .send()
            .await
            .expect("websocket route request should succeed");

        assert!(
            response.status().is_client_error() || response.status().is_redirection(),
            "unexpected status for websocket handshake probe: {}",
            response.status()
        );
    }

    #[tokio::test]
    async fn worker_completes_incomplete_strategy_turn_with_clarification() {
        let server = TestServer::spawn_with_worker().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;
        let mut response = client
            .get(server.url(&format!("/v1/sessions/{}/events", created.session_id)))
            .send()
            .await
            .expect("events request should succeed")
            .error_for_status()
            .expect("events request should return success");
        let mut buffer = String::new();
        let _snapshot_frame = next_sse_frame(&mut response, &mut buffer).await;

        let accepted = client
            .post(server.url(&format!("/v1/sessions/{}/turn", created.session_id)))
            .json(&SubmitTurnRequest {
                message_type: TurnMessageType::Intent,
                text: "Create a simple SMA crossover strategy for EURUSD H1 with stop-loss 50 pips."
                    .to_string(),
                context: TurnContext::default(),
            })
            .send()
            .await
            .expect("turn request should succeed")
            .error_for_status()
            .expect("turn request should return success")
            .json::<SubmitTurnResponse>()
            .await
            .expect("turn response should parse");

        let mut saw_clarification = false;
        for _ in 0..8 {
            let frame = next_sse_frame(&mut response, &mut buffer).await;
            if frame.contains("event: clarification_question") {
                saw_clarification = true;
                break;
            }
        }

        let task = wait_for_task_completion(&client, &server, &accepted.task_id).await;

        assert!(saw_clarification, "expected clarification event");
        assert_eq!(task.result_type, Some(TaskResultType::Clarification));
        assert_eq!(task.payload["status"], "INCOMPLETE");
        assert_eq!(task.payload["missing_fields"][0], "action");
        assert_eq!(
            task.payload["next_question"],
            "What trading action? (BUY or SELL)"
        );
    }

    #[tokio::test]
    async fn worker_generates_code_and_validation_when_spec_is_complete() {
        let server = TestServer::spawn_with_worker().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;
        let mut response = client
            .get(server.url(&format!("/v1/sessions/{}/events", created.session_id)))
            .send()
            .await
            .expect("events request should succeed")
            .error_for_status()
            .expect("events request should return success");
        let mut buffer = String::new();
        let _snapshot_frame = next_sse_frame(&mut response, &mut buffer).await;

        let accepted = client
            .post(server.url(&format!("/v1/sessions/{}/turn", created.session_id)))
            .json(&SubmitTurnRequest {
                message_type: TurnMessageType::Intent,
                text: "Build a BUY EURUSD H1 strategy when 50 SMA crosses above 200 SMA with reverse cross exit and stop-loss 50 pips."
                    .to_string(),
                context: TurnContext::default(),
            })
            .send()
            .await
            .expect("turn request should succeed")
            .error_for_status()
            .expect("turn request should return success")
            .json::<SubmitTurnResponse>()
            .await
            .expect("turn response should parse");

        let mut saw_generated_code = false;
        let mut saw_validation_feedback = false;
        for _ in 0..12 {
            let frame = next_sse_frame(&mut response, &mut buffer).await;
            if frame.contains("event: generated_code") {
                saw_generated_code = true;
            }
            if frame.contains("event: validation_feedback")
                && frame.contains("\"stage\":\"static_analysis\"")
            {
                saw_validation_feedback = true;
            }
            if saw_generated_code && saw_validation_feedback {
                break;
            }
        }

        let task = wait_for_task_completion(&client, &server, &accepted.task_id).await;

        assert!(saw_generated_code, "expected generated_code event");
        assert!(
            saw_validation_feedback,
            "expected static-analysis validation_feedback event"
        );
        assert_eq!(task.result_type, Some(TaskResultType::Generation));
        assert_eq!(task.payload["status"], "COMPLETE");
        assert_eq!(task.payload["generation"]["skeleton_type"], "sma_crossover");
        assert_eq!(task.payload["analysis"]["passed"], true);
        assert_eq!(task.payload["ready_for_compile"], true);
        let generated_code = task.payload["generation"]["code"]
            .as_str()
            .expect("generated code should be a string");
        assert!(generated_code.contains("OnTick()"));
        assert!(generated_code.contains("Requested entry condition"));
    }

    async fn wait_for_task_completion(
        client: &Client,
        server: &TestServer,
        task_id: &str,
    ) -> TaskStatusResponse {
        for _ in 0..20 {
            let task = client
                .get(server.url(&format!("/v1/tasks/{task_id}")))
                .send()
                .await
                .expect("task request should succeed")
                .error_for_status()
                .expect("task request should return success")
                .json::<TaskStatusResponse>()
                .await
                .expect("task response should parse");
            if task.status == TaskStatus::Completed || task.status == TaskStatus::Failed {
                return task;
            }
            sleep(Duration::from_millis(50)).await;
        }

        panic!("task `{task_id}` did not reach a terminal state in time");
    }
}
