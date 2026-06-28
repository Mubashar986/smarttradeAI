use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use runtime::{
    compile_mql5_async, save_strategy_async, CompileResult, ContentBlock, ConversationMessage,
    ConversationRuntime, PermissionMode, PermissionPolicy, SaveStrategyRequest, Session,
    SmartTradeToolConfig, SmartTradeToolExecutor, TokenUsage,
};
use serde_json::json;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::llm_bridge::LlmBridge;
use crate::mql5_extractor::extract_mql5_code;
use crate::middleware::auth::AuthClaims;
use crate::state::{
    AppState, ApiResult, ErrorResponse, SendMessageRequest, SessionEvent, SessionId,
    SubmitTurnRequest, SubmitTurnResponse, TaskId, TaskResultType, TaskStatus, TaskStatusResponse,
    TurnContext, TurnMessageType, TurnRequest, TurnTask, not_found,
};

const DEFAULT_MAX_TURN_ITERATIONS: usize = 8;
const MAX_CODEGEN_COMPILE_ATTEMPTS: usize = 2;
const DEFAULT_PROCESS_TURN_TIMEOUT_SECS: u64 = 900;

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
            let turn_timeout = process_turn_timeout();
            let result = timeout(turn_timeout, process_turn(state.clone(), request.clone()))
                .await
                .map_err(|_| {
                    format!(
                        "turn processing timed out after {} seconds",
                        turn_timeout.as_secs()
                    )
                })
                .and_then(|result| result);

            if let Err(error) = result {
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
                        message: panic_msg.clone(),
                    },
                )
                .await;
                broadcast_event(
                    &watchdog_state,
                    &session_id,
                    SessionEvent::TurnError {
                        session_id: session_id.clone(),
                        error: panic_msg,
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
    let system_prompt = smarttrade_system_prompt();

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
    )
    .with_max_iterations(max_turn_iterations());

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
    let mut response_text = summary
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
    let mut total_iterations = summary.iterations;
    let mut total_usage = summary.usage;

    // 7a. Extract MQL5 code block from the assistant response.
    let mql5_code = extract_mql5_code(&response_text);
    let has_code = mql5_code.is_some();
    let mut final_code = mql5_code.clone();
    if let Some(ref code) = mql5_code {
        broadcast_event(
            &state,
            &request.session_id,
            SessionEvent::DraftGeneratedCode {
                session_id: request.session_id.clone(),
                task_id: request.task_id.clone(),
                content: code.clone(),
            },
        )
        .await;
    }

    // 8. Compile/fix loop (only if code was extracted). The LLM generates code;
    // the server owns compilation so provider tool loops cannot retry forever.
    let mut compile_status = "NOT_ATTEMPTED";
    let mut strategy_id: Option<String> = None;
    let mut compile_errors: Vec<String> = Vec::new();

    if let Some(ref code) = mql5_code {
        let mut current_code = code.clone();

        for attempt in 1..=MAX_CODEGEN_COMPILE_ATTEMPTS {
            broadcast_status(
                &state,
                &request.session_id,
                &request.task_id,
                "compiling",
                &format!("compile attempt {attempt}/{MAX_CODEGEN_COMPILE_ATTEMPTS}"),
            )
            .await;

            let result = compile_mql5_async(&current_code, &request.session_id, attempt as u64)
                .await;
            compile_errors = compile_messages(&result);

            if result.source == "stub" {
                compile_status = "STUB_SKIPPED";
                break;
            }

            if compiler_unavailable(&result) {
                compile_status = "COMPILER_UNAVAILABLE";
                break;
            }

            if result.success {
                compile_status = "COMPILED";
                break;
            }

            if attempt >= MAX_CODEGEN_COMPILE_ATTEMPTS {
                compile_status = "FAILED";
                break;
            }

            // Build feedback and add to session
            let feedback = build_compile_feedback(&compile_errors);

            // Consume runtime into session, add feedback, rebuild runtime
            let mut session = runtime.into_session();
            session.messages.push(ConversationMessage::user_text(feedback));
            runtime = build_runtime_from_session(session, &state);

            // Re-run conversation
            broadcast_status(
                &state,
                &request.session_id,
                &request.task_id,
                "llm_turn",
                "asking LLM to fix compilation errors",
            )
            .await;

            let (retry_result, retry_runtime) = tokio::task::spawn_blocking(move || {
                let result = runtime.run_turn_headless();
                (result, runtime)
            })
            .await
            .map_err(|e| format!("retry turn loop panicked: {e}"))?;

            let retry_summary = retry_result.map_err(|e| {
                tracing::error!(
                    task_id = %request.task_id,
                    error = %e,
                    "retry LLM turn failed"
                );
                format!("retry LLM turn failed: {e}")
            })?;

            runtime = retry_runtime;

            tracing::info!(
                task_id = %request.task_id,
                iterations = retry_summary.iterations,
                "retry LLM turn completed"
            );
            total_iterations += retry_summary.iterations;
            add_usage(&mut total_usage, retry_summary.usage);

            // Extract new code from retry
            let retry_text = retry_summary
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

            if let Some(new_code) = extract_mql5_code(&retry_text) {
                current_code = new_code.clone();
                final_code = Some(new_code);
                response_text = retry_text;
                broadcast_event(
                    &state,
                    &request.session_id,
                    SessionEvent::DraftGeneratedCode {
                        session_id: request.session_id.clone(),
                        task_id: request.task_id.clone(),
                        content: current_code.clone(),
                    },
                )
                .await;
                tracing::info!(task_id = %request.task_id, "retry extracted new MQL5 code");
            } else {
                compile_status = "FAILED";
                break;
            }
        }

        // Save strategy on successful compilation
        if compile_status == "COMPILED" {
            broadcast_status(
                &state,
                &request.session_id,
                &request.task_id,
                "saving",
                "saving compiled strategy",
            )
            .await;

            let save_request = SaveStrategyRequest {
                strategy_name: "Generated Strategy".to_string(),
                code: current_code.clone(),
                explanation: response_text.chars().take(200).collect::<String>(),
                status: "COMPILED".to_string(),
                session_id: request.session_id.clone(),
                user_id: request.context.user_id.clone().unwrap_or_default(),
                pair: "".to_string(),
                timeframe: "".to_string(),
            };

            let save_result = save_strategy_async(&save_request).await;
            if save_result.success {
                strategy_id = save_result.strategy_id.clone();
                tracing::info!(
                    task_id = %request.task_id,
                    strategy_id = %strategy_id.as_deref().unwrap_or("unknown"),
                    "strategy saved successfully"
                );
            } else {
                tracing::warn!(
                    task_id = %request.task_id,
                    error = %save_result.error.as_deref().unwrap_or("unknown"),
                    "strategy save failed"
                );
            }
        }

        if matches!(compile_status, "COMPILED" | "STUB_SKIPPED") {
            broadcast_event(
                &state,
                &request.session_id,
                SessionEvent::GeneratedCode {
                    session_id: request.session_id.clone(),
                    task_id: request.task_id.clone(),
                    content: current_code,
                },
            )
            .await;
        }
    }

    // 9. Write the updated session back to AppState.
    let updated_session = runtime.into_session();
    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(&request.session_id) {
            session.conversation = updated_session;
        }
    }

    // 10. Broadcast the assistant reply.
    let reply = ConversationMessage::assistant(vec![ContentBlock::Text {
        text: response_text.clone(),
    }]);
    {
        let sessions = state.sessions.read().await;
        if let Some(session) = sessions.get(&request.session_id) {
            session.broadcast(SessionEvent::AssistantReply {
                session_id: request.session_id.clone(),
                message: reply,
            });
        }
    }

    // 11. Complete the task.
    let mut payload = json!({
        "status": "completed",
        "iterations": total_iterations,
        "usage": {
            "input_tokens": total_usage.input_tokens,
            "output_tokens": total_usage.output_tokens,
            "cache_creation_input_tokens": total_usage.cache_creation_input_tokens,
            "cache_read_input_tokens": total_usage.cache_read_input_tokens,
        },
        "response_preview": response_text.chars().take(200).collect::<String>(),
        "has_code": has_code,
        "compile_status": compile_status,
    });
    if let Some(code) = final_code {
        payload["code"] = json!(code);
    }
    if let Some(id) = &strategy_id {
        payload["strategy_id"] = json!(id);
    }
    if !compile_errors.is_empty() && matches!(compile_status, "FAILED" | "COMPILER_UNAVAILABLE") {
        payload["compile_errors"] = json!(compile_errors);
    }
    state
        .complete_task(
            &request.task_id,
            TaskResultType::Generation,
            payload,
        )
        .await;

    // 12. Broadcast final status.
    let final_status = match compile_status {
        "COMPILED" => "compilation_complete",
        "FAILED" => "compilation_failed",
        "STUB_SKIPPED" => "compilation_stub",
        "COMPILER_UNAVAILABLE" => "compilation_unavailable",
        _ => "generation_complete",
    };
    broadcast_status(
        &state,
        &request.session_id,
        &request.task_id,
        final_status,
        match compile_status {
            "COMPILED" => "strategy compiled and saved",
            "FAILED" => "compilation failed after all retries",
            "STUB_SKIPPED" => "compilation skipped (stub mode)",
            "COMPILER_UNAVAILABLE" => "compiler service unavailable",
            _ => "LLM conversation turn finished",
        },
    )
    .await;

    // 13. Broadcast turn complete.
    broadcast_event(
        &state,
        &request.session_id,
        SessionEvent::TurnComplete {
            session_id: request.session_id.clone(),
            iterations: total_iterations,
        },
    )
    .await;

    tracing::info!(
        task_id = %request.task_id,
        session_id = %request.session_id,
        compile_status = %compile_status,
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

/// Build a feedback message from compiler diagnostics to send back to the LLM.
fn build_compile_feedback(errors: &[String]) -> String {
    let mut msg = String::from(
        "The MQL5 compiler reported the following diagnostics:\n\n",
    );
    for err in errors {
        msg.push_str(&format!("- {err}\n"));
    }
    msg.push_str(
        "\nPlease fix these diagnostics and regenerate the complete MQL5 code \
         with all necessary #property tags, OnInit(), OnDeinit(), and OnTick() handlers.",
    );
    msg
}

/// Build a new ConversationRuntime from an existing session for retry purposes.
fn build_runtime_from_session(
    session: Session,
    state: &AppState,
) -> ConversationRuntime<LlmBridge, SmartTradeToolExecutor> {
    let max_tokens = api::max_tokens_for_model(&state.llm_model);
    let bridge = LlmBridge::new(state.provider.clone(), state.llm_model.clone(), max_tokens);
    let executor = SmartTradeToolExecutor::with_config(SmartTradeToolConfig::from_env());
    let policy = PermissionPolicy::new(PermissionMode::Allow);
    ConversationRuntime::new(session, bridge, executor, policy, smarttrade_system_prompt())
        .with_max_iterations(max_turn_iterations())
}

fn smarttrade_system_prompt() -> Vec<String> {
    vec![
        "You are an expert MQL5 trading strategy assistant. You help users design, \
         build, and refine automated trading strategies (Expert Advisors) for \
         MetaTrader 5. Use the provided tools to classify intents, check for \
         missing details, search the knowledge base, and run static analysis. \
         If you already classified the intent or detected parameters, skip calling \
         classify_intent and detect_ambiguity tools on compilation retries. Focus \
         only on correcting the compiler errors. \
         When generating a strategy, you MUST \
         write the COMPLETE MQL5 code from scratch, including all necessary \
         #property tags, OnInit(), OnDeinit(), and OnTick() event handlers. \
         Use MQL5-native market data and indicator-handle patterns: SymbolInfoDouble \
         for bid/ask, CopyRates/CopyBuffer for time-series and indicators, CTrade or \
         MqlTradeRequest for orders, and never MQL4 globals or arrays such as Bid, Ask, \
         Time[0], Open[], Close[], MarketInfo(), or shifted direct indicator calls. \
         Return final source in a fenced ```mql5 code block. The server will \
         compile and save the strategy after your response, so do not attempt \
         to compile or persist it through tools. Do not rely on any external \
         templates or skeletons."
            .to_string(),
    ]
}

fn max_turn_iterations() -> usize {
    std::env::var("CLAW_MAX_TURN_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MAX_TURN_ITERATIONS)
}

fn process_turn_timeout() -> Duration {
    Duration::from_secs(
        std::env::var("PROCESS_TURN_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_PROCESS_TURN_TIMEOUT_SECS),
    )
}

fn compile_messages(result: &CompileResult) -> Vec<String> {
    let mut messages = result
        .errors
        .iter()
        .map(|error| error.message.clone())
        .collect::<Vec<_>>();

    messages.extend(
        result
            .warnings
            .iter()
            .map(|warning| format!("warning: {}", warning.message)),
    );

    if messages.is_empty() {
        if let Some(message) = &result.message {
            messages.push(message.clone());
        }
    }
    if messages.is_empty() {
        if let Some(note) = &result.note {
            messages.push(note.clone());
        }
    }
    if messages.is_empty() {
        messages.push(format!(
            "Compiler returned status {} from source {} without detailed diagnostics.",
            result.status.as_deref().unwrap_or("UNKNOWN"),
            result.source
        ));
    }

    messages
}

fn compiler_unavailable(result: &CompileResult) -> bool {
    result.source == "c3_error"
        || matches!(
            result.status.as_deref(),
            Some(
                "METAEDITOR_NOT_FOUND"
                    | "DIRECTORY_CREATE_FAILED"
                    | "FILE_WRITE_FAILED"
                    | "TIMEOUT"
                    | "SUBPROCESS_FAILED"
            )
        )
}

#[cfg(test)]
mod tests {
    use super::{compile_messages, compiler_unavailable};
    use runtime::{CompileResult, CompilerMessage};

    fn compile_result(
        status: Option<&str>,
        errors: Vec<&str>,
        warnings: Vec<&str>,
        message: Option<&str>,
        note: Option<&str>,
    ) -> CompileResult {
        CompileResult {
            success: false,
            status: status.map(ToOwned::to_owned),
            retry: 1,
            max_retries: Some(2),
            errors: errors
                .into_iter()
                .map(|message| CompilerMessage {
                    message: message.to_string(),
                })
                .collect(),
            warnings: warnings
                .into_iter()
                .map(|message| CompilerMessage {
                    message: message.to_string(),
                })
                .collect(),
            source: "metaeditor".to_string(),
            note: note.map(ToOwned::to_owned),
            message: message.map(ToOwned::to_owned),
            ex5_base64: None,
        }
    }

    #[test]
    fn compile_messages_includes_warnings_with_errors() {
        let result = compile_result(
            Some("COMPILE_FAILED"),
            vec!["missing semicolon"],
            vec!["unused variable"],
            None,
            None,
        );

        assert_eq!(
            compile_messages(&result),
            vec!["missing semicolon", "warning: unused variable"]
        );
    }

    #[test]
    fn compile_messages_falls_back_when_diagnostics_are_empty() {
        let result = compile_result(
            Some("COMPILE_FAILED"),
            Vec::new(),
            Vec::new(),
            Some("compile failed"),
            Some("no log found"),
        );

        assert_eq!(compile_messages(&result), vec!["compile failed"]);
    }

    #[test]
    fn artifact_missing_is_retryable_compile_failure() {
        let result = compile_result(
            Some("ARTIFACT_MISSING"),
            vec!["Compiler reported success without a .ex5 artifact."],
            Vec::new(),
            None,
            None,
        );

        assert!(!compiler_unavailable(&result));
    }

    #[test]
    fn infrastructure_failures_remain_compiler_unavailable() {
        for status in [
            "METAEDITOR_NOT_FOUND",
            "DIRECTORY_CREATE_FAILED",
            "FILE_WRITE_FAILED",
            "TIMEOUT",
            "SUBPROCESS_FAILED",
        ] {
            let result = compile_result(Some(status), Vec::new(), Vec::new(), None, None);
            assert!(compiler_unavailable(&result), "{status} should be unavailable");
        }
    }
}

fn add_usage(total: &mut TokenUsage, next: TokenUsage) {
    total.input_tokens = total.input_tokens.saturating_add(next.input_tokens);
    total.output_tokens = total.output_tokens.saturating_add(next.output_tokens);
    total.cache_creation_input_tokens = total
        .cache_creation_input_tokens
        .saturating_add(next.cache_creation_input_tokens);
    total.cache_read_input_tokens = total
        .cache_read_input_tokens
        .saturating_add(next.cache_read_input_tokens);
}
