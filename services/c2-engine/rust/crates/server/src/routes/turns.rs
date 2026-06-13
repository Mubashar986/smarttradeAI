use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use runtime::{
    classify_intent, detect_ambiguity, extract_strategy_spec, generate_strategy_code,
    run_static_analysis, AmbiguityStatus, ContentBlock, ConversationMessage, StrategyIntent,
};
use serde_json::json;
use tokio::sync::mpsc;

use crate::middleware::auth::AuthClaims;
use crate::state::{
    AppState, ApiError, ApiResult, ErrorResponse, SendMessageRequest, SessionEvent, SessionId,
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
