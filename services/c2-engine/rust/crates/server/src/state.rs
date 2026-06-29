use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use api::ProviderClient;

use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::Json;
use runtime::{ConversationMessage, Session as RuntimeSession};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

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
    pub(crate) fn queued(
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
    pub(crate) turn_tx: mpsc::UnboundedSender<TurnRequest>,
    pub llm_model: String,
    pub(crate) provider: Arc<ProviderClient>,
    pub pool: Option<sqlx::PgPool>,
}

impl AppState {
    #[must_use]
    pub fn new(pool: Option<sqlx::PgPool>) -> (Self, mpsc::UnboundedReceiver<TurnRequest>) {
        let (turn_tx, turn_rx) = mpsc::unbounded_channel();

        let llm_model = std::env::var("LLM_MODEL")
            .or_else(|_| std::env::var("CLAW_MODEL"))
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
        let provider = Arc::new(
            ProviderClient::from_model(&llm_model)
                .expect("LLM provider must be configured (check API key env vars)"),
        );

        (
            Self {
                sessions: Arc::new(RwLock::new(HashMap::new())),
                tasks: Arc::new(RwLock::new(HashMap::new())),
                next_session_id: Arc::new(AtomicU64::new(1)),
                next_task_id: Arc::new(AtomicU64::new(1)),
                turn_locks: Arc::new(RwLock::new(HashMap::new())),
                clarification_rounds: Arc::new(RwLock::new(HashMap::new())),
                turn_tx,
                llm_model,
                provider,
                pool,
            },
            turn_rx,
        )
    }

    pub(crate) fn allocate_session_id(&self) -> SessionId {
        let id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        format!("session-{id}")
    }

    pub(crate) fn allocate_task_id(&self) -> TaskId {
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
        Self::new(None).0
    }
}

#[derive(Clone)]
pub struct Session {
    pub id: SessionId,
    pub created_at: u64,
    pub conversation: RuntimeSession,
    pub(crate) events: broadcast::Sender<SessionEvent>,
}

impl Session {
    pub(crate) fn new(id: SessionId) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            id,
            created_at: unix_timestamp_millis(),
            conversation: RuntimeSession::new(),
            events,
        }
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
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
    DraftGeneratedCode {
        session_id: SessionId,
        task_id: TaskId,
        content: String,
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
            Self::DraftGeneratedCode { .. } => "draft_generated_code",
            Self::GeneratedCode { .. } => "generated_code",
            Self::Error { .. } => "error",
        }
    }

    pub(crate) fn to_sse_event(&self) -> Result<Event, serde_json::Error> {
        Ok(Event::default()
            .event(self.event_name())
            .data(serde_json::to_string(self)?))
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}

pub(crate) type ApiError = (StatusCode, Json<ErrorResponse>);
pub(crate) type ApiResult<T> = Result<T, ApiError>;

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

impl StrategyRecord {
    pub(crate) fn summary(self) -> StrategySummary {
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

pub(crate) fn unix_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_millis() as u64
}

pub(crate) fn internal_error(message: String) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: message }),
    )
}

pub(crate) fn not_found(message: String) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: message }),
    )
}
