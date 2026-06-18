use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use runtime::{
    ContentBlock, ConversationMessage, ConversationRuntime, PermissionMode, PermissionPolicy,
    SmartTradeToolConfig, SmartTradeToolExecutor,
};
use serde_json::json;
use tokio::sync::mpsc;

use crate::llm_bridge::LlmBridge;
use crate::middleware::auth::AuthClaims;
use crate::state::{
    AppState, ApiResult, ErrorResponse, SendMessageRequest, SessionEvent, SessionId,
    SubmitTurnRequest, SubmitTurnResponse, TaskId, TaskResultType, TaskStatus, TaskStatusResponse,
    TurnContext, TurnMessageType, TurnRequest, TurnTask, not_found,
};

pub(crate) async fn send_message(
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

pub(crate) async fn send_turn(
    State(state): State<AppState>,
    claims: Option<Extension<AuthClaims>>,
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

    tracing::info!(
        task_id = %task_id,
        session_id = %session_id,
        "turn enqueued"
    );

    Ok(task_id)
}

pub(crate) async fn get_task(
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

pub async fn run_turn_worker(state: AppState, mut turn_rx: mpsc::UnboundedReceiver<TurnRequest>) {
    tracing::info!("turn worker started — waiting for requests");
    while let Some(request) = turn_rx.recv().await {
        let state = state.clone();
        let task_id = request.task_id.clone();
        let session_id = request.session_id.clone();

        tracing::info!(
            task_id = %task_id,
            session_id = %session_id,
            message_type = %request.message_type.as_str(),
            "turn worker picked up request"
        );

        let watchdog_state = state.clone();
        let handle = tokio::spawn(async move {
            if let Err(error) = process_turn(state.clone(), request.clone()).await {
                tracing::error!(
                    task_id = %request.task_id,
                    session_id = %request.session_id,
                    error = %error,
                    "process_turn failed"
                );
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

        // Check if the spawned task panicked. If it did, mark the task as
        // failed so it doesn't remain in "running" forever.
        tokio::spawn(async move {
            if let Err(join_error) = handle.await {
                let panic_msg = if join_error.is_panic() {
                    format!("turn task panicked: {join_error}")
                } else {
                    format!("turn task cancelled: {join_error}")
                };
                tracing::error!(
                    task_id = %task_id,
                    session_id = %session_id,
                    error = %panic_msg,
                    "spawned turn task did not complete normally"
                );
                watchdog_state.fail_task(&task_id, panic_msg.clone()).await;
                broadcast_event(
                    &watchdog_state,
                    &session_id,
                    SessionEvent::Error {
                        session_id: session_id.clone(),
                        task_id: task_id.clone(),
                        message: panic_msg,
                    },
                )
                .await;
            }
        });
    }
    tracing::warn!("turn worker channel closed — no more turns will be processed");
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

    // 1. Snapshot the current session conversation.
    let session_snapshot = {
        let sessions = state.sessions.read().await;
        let session = sessions
            .get(&request.session_id)
            .ok_or_else(|| format!("session `{}` not found", request.session_id))?;
        session.conversation.clone()
    };

    // 2. Build the LLM bridge from the provider stored in AppState.
    let max_tokens = api::max_tokens_for_model(&state.llm_model);
    let bridge = LlmBridge::new(state.provider.clone(), state.llm_model.clone(), max_tokens);

    tracing::info!(
        task_id = %request.task_id,
        model = %state.llm_model,
        max_tokens = max_tokens,
        message_count = session_snapshot.messages.len(),
        "starting LLM turn"
    );

    // 3. Build the tool executor.
    let executor = SmartTradeToolExecutor::with_config(SmartTradeToolConfig::from_env());

    // 4. Build permission policy (allow everything in headless server mode).
    let policy = PermissionPolicy::new(PermissionMode::Allow);

    // 5. System prompt.
    let system_prompt = vec![
        "You are an expert MQL5 trading strategy assistant. You help users design, \
         build, and refine automated trading strategies (Expert Advisors) for \
         MetaTrader 5. Use the provided tools to classify intents, check for \
         missing details, search the knowledge base, run static analysis, \
         compile, and save strategies. When generating a strategy, you MUST \
         write the COMPLETE MQL5 code from scratch, including all necessary \
         #property tags, OnInit(), OnDeinit(), and OnTick() event handlers. \
         Do not rely on any external templates or skeletons."
            .to_string(),
    ];

    // 6. Construct and run the conversation runtime.
    broadcast_status(
        &state,
        &request.session_id,
        &request.task_id,
        "llm_turn",
        "running LLM conversation turn",
    )
    .await;

    let mut runtime = ConversationRuntime::new(
        session_snapshot,
        bridge,
        executor,
        policy,
        system_prompt,
    );

    let (summary_result, mut runtime) = tokio::task::spawn_blocking(move || {
        let result = runtime.run_turn_headless();
        (result, runtime)
    })
    .await
    .map_err(|e| format!("turn loop panicked or failed to join: {e}"))?;

    let summary = summary_result.map_err(|e| {
        tracing::error!(
            task_id = %request.task_id,
            error = %e,
            "LLM turn failed"
        );
        format!("LLM turn failed: {e}")
    })?;

    tracing::info!(
        task_id = %request.task_id,
        iterations = summary.iterations,
        input_tokens = summary.usage.input_tokens,
        output_tokens = summary.usage.output_tokens,
        assistant_messages = summary.assistant_messages.len(),
        tool_results = summary.tool_results.len(),
        "LLM turn completed"
    );

    // 7. Extract the response text from the last assistant message.
    let response_text = summary
        .assistant_messages
        .last()
        .map(|msg| {
            msg.blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    // 8. Write the updated session back to AppState.
    let updated_session = runtime.into_session();
    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(&request.session_id) {
            session.conversation = updated_session;
        }
    }

    // 9. Broadcast the assistant reply.
    let reply = ConversationMessage::assistant(vec![ContentBlock::Text {
        text: response_text.clone(),
    }]);
    // Note: we don't push to session.conversation again — it's already there
    // from the runtime. We only broadcast the event for SSE listeners.
    {
        let sessions = state.sessions.read().await;
        if let Some(session) = sessions.get(&request.session_id) {
            session.broadcast(SessionEvent::AssistantReply {
                session_id: request.session_id.clone(),
                message: reply,
            });
        }
    }

    // 10. Complete the task.
    state
        .complete_task(
            &request.task_id,
            TaskResultType::Generation,
            json!({
                "status": "completed",
                "iterations": summary.iterations,
                "usage": {
                    "input_tokens": summary.usage.input_tokens,
                    "output_tokens": summary.usage.output_tokens,
                    "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                    "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
                },
                "response_preview": response_text.chars().take(200).collect::<String>(),
            }),
        )
        .await;
    broadcast_status(
        &state,
        &request.session_id,
        &request.task_id,
        "generation_complete",
        "LLM conversation turn finished",
    )
    .await;

    // 11. Broadcast turn complete.
    broadcast_event(
        &state,
        &request.session_id,
        SessionEvent::TurnComplete {
            session_id: request.session_id.clone(),
            iterations: summary.iterations,
        },
    )
    .await;

    tracing::info!(
        task_id = %request.task_id,
        session_id = %request.session_id,
        "turn processing complete"
    );

    Ok(())
}



#[allow(dead_code)]
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
