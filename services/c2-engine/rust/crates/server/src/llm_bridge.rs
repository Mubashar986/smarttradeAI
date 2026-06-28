//! Bridge between the runtime crate's synchronous [`ApiClient`] trait and the
//! api crate's async [`ProviderClient`].  Translates between their message and
//! event types, and uses `block_in_place` to call the async streaming API from
//! the synchronous `stream()` method.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use api::{
    ContentBlockDelta, InputContentBlock, InputMessage, MessageRequest, OutputContentBlock,
    ProviderClient, StreamEvent, ToolDefinition, ToolResultContentBlock,
};
use runtime::{
    ApiClient, ApiRequest, AssistantEvent, ContentBlock, ConversationMessage, MessageRole,
    RuntimeError, TokenUsage,
};
use serde_json::{Value, json};

/// Default timeout for the entire LLM streaming call (connect + drain).
const DEFAULT_LLM_STREAM_TIMEOUT_SECS: u64 = 600;

/// Timeout for a single SSE event within the drain loop. If no event arrives
/// within this window, the stream is considered dead.
const DEFAULT_LLM_EVENT_TIMEOUT_SECS: u64 = 300;

/// Retry only streams that stall before the first provider event. Retrying
/// after partial output risks duplicated text/tool calls.
const DEFAULT_LLM_INITIAL_STALL_RETRIES: u32 = 1;

#[derive(Debug, Clone, Copy)]
struct StreamPolicy {
    overall_timeout: Duration,
    event_timeout: Duration,
    initial_stall_retries: u32,
}

impl StreamPolicy {
    fn from_env() -> Self {
        Self {
            overall_timeout: Duration::from_secs(read_positive_env_u64(
                "LLM_STREAM_TIMEOUT_SECS",
                DEFAULT_LLM_STREAM_TIMEOUT_SECS,
            )),
            event_timeout: Duration::from_secs(read_positive_env_u64(
                "LLM_EVENT_TIMEOUT_SECS",
                DEFAULT_LLM_EVENT_TIMEOUT_SECS,
            )),
            initial_stall_retries: read_positive_env_u64(
                "LLM_INITIAL_STALL_RETRIES",
                u64::from(DEFAULT_LLM_INITIAL_STALL_RETRIES),
            ) as u32,
        }
    }
}

fn read_positive_env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn read_optional_env_f32(key: &str) -> Option<f32> {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
}

// ---------------------------------------------------------------------------
// LlmBridge
// ---------------------------------------------------------------------------

pub(crate) struct LlmBridge {
    provider: Arc<ProviderClient>,
    model: String,
    max_tokens: u32,
}

impl LlmBridge {
    pub(crate) fn new(provider: Arc<ProviderClient>, model: String, max_tokens: u32) -> Self {
        Self {
            provider,
            model,
            max_tokens,
        }
    }
}

// ---------------------------------------------------------------------------
// Pending tool accumulator (used while draining the stream)
// ---------------------------------------------------------------------------

struct PendingTool {
    id: String,
    name: String,
    input_buffer: String,
}

// ---------------------------------------------------------------------------
// ApiClient implementation
// ---------------------------------------------------------------------------

impl ApiClient for LlmBridge {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        // --- translate input ------------------------------------------------
        let system = if request.system_prompt.is_empty() {
            None
        } else {
            Some(request.system_prompt.join("\n"))
        };

        let messages = translate_messages(&request.messages);
        let tools = tool_definitions();

        let api_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: read_optional_env_f32("CLAW_TEMPERATURE"),
            messages,
            system,
            tools: if tools.is_empty() { None } else { Some(tools) },
            tool_choice: None,
            stream: true,
        };

        let model_name = self.model.clone();
        let max_tokens = self.max_tokens;
        let policy = StreamPolicy::from_env();

        // --- bridge async → sync --------------------------------------------
        let provider = self.provider.clone();
        let events = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                tracing::info!(
                    model = %model_name,
                    max_tokens = max_tokens,
                    message_count = api_request.messages.len(),
                    tool_count = api_request.tools.as_ref().map_or(0, |t| t.len()),
                    stream_timeout_secs = policy.overall_timeout.as_secs(),
                    event_timeout_secs = policy.event_timeout.as_secs(),
                    initial_stall_retries = policy.initial_stall_retries,
                    "sending LLM streaming request"
                );

                // Wrap the entire call in an overall timeout.
                let result = tokio::time::timeout(
                    policy.overall_timeout,
                    drain_stream_with_retry(&provider, &api_request, policy),
                )
                .await;

                match result {
                    Ok(inner) => inner,
                    Err(_elapsed) => {
                        tracing::error!(
                            timeout_secs = policy.overall_timeout.as_secs(),
                            "LLM streaming call timed out"
                        );
                        Err(RuntimeError::new(format!(
                            "LLM API call timed out after {}s",
                            policy.overall_timeout.as_secs()
                        )))
                    }
                }
            })
        })?;

        tracing::info!(
            event_count = events.len(),
            "LLM streaming response received"
        );
        Ok(events)
    }
}

async fn drain_stream_with_retry(
    provider: &ProviderClient,
    api_request: &MessageRequest,
    policy: StreamPolicy,
) -> Result<Vec<AssistantEvent>, RuntimeError> {
    let mut retry_count = 0;
    loop {
        match drain_stream(provider, api_request, policy.event_timeout).await {
            Err(error)
                if is_initial_stream_stall(&error)
                    && retry_count < policy.initial_stall_retries =>
            {
                retry_count += 1;
                tracing::warn!(
                    retry = retry_count,
                    max_retries = policy.initial_stall_retries,
                    error = %error,
                    "retrying LLM stream after pre-token stall"
                );
            }
            result => return result,
        }
    }
}

fn is_initial_stream_stall(error: &RuntimeError) -> bool {
    error
        .to_string()
        .contains("LLM stream stalled before first event")
}

/// Drains the SSE stream from the provider, applying a per-event timeout.
async fn drain_stream(
    provider: &ProviderClient,
    api_request: &MessageRequest,
    event_timeout: Duration,
) -> Result<Vec<AssistantEvent>, RuntimeError> {
    let mut stream = provider
        .stream_message(api_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to open LLM stream");
            RuntimeError::new(format!("API stream error: {e}"))
        })?;
    let request_id = stream.request_id().map(ToOwned::to_owned);
    tracing::debug!(request_id = ?request_id, "LLM stream opened");
    tracing::info!(request_id = ?request_id, "llm_stream_opened");

    let mut events: Vec<AssistantEvent> = Vec::new();
    let mut pending_tools: BTreeMap<u32, PendingTool> = BTreeMap::new();

    // Usage accumulators – input-side tokens arrive in MessageStart,
    // output tokens arrive in MessageDelta.
    let mut input_tokens: u32 = 0;
    let mut cache_creation_input_tokens: u32 = 0;
    let mut cache_read_input_tokens: u32 = 0;
    let mut event_count: u32 = 0;

    loop {
        // Per-event timeout: if the provider goes silent, bail out.
        let next = tokio::time::timeout(
            event_timeout,
            stream.next_event(),
        )
        .await;

        let event = match next {
            Ok(result) => result.map_err(|e| {
                tracing::error!(error = %e, events_so_far = event_count, "stream event error");
                RuntimeError::new(format!("stream event error: {e}"))
            })?,
            Err(_elapsed) => {
                let stall_message = if event_count == 0 {
                    format!(
                        "LLM stream stalled before first event: no event received within {}s",
                        event_timeout.as_secs()
                    )
                } else {
                    format!(
                        "LLM stream stalled after {event_count} events: no event received within {}s",
                        event_timeout.as_secs()
                    )
                };
                tracing::warn!(
                    timeout_secs = event_timeout.as_secs(),
                    events_received = event_count,
                    request_id = ?request_id,
                    "LLM stream stalled — no event received within timeout"
                );
                return Err(RuntimeError::new(stall_message));
            }
        };

        let Some(event) = event else {
            break; // Stream ended cleanly.
        };

        event_count += 1;

        match event {
            // Capture input-side usage from the opening envelope.
            StreamEvent::MessageStart(start) => {
                input_tokens = start.message.usage.input_tokens;
                cache_creation_input_tokens =
                    start.message.usage.cache_creation_input_tokens;
                cache_read_input_tokens =
                    start.message.usage.cache_read_input_tokens;
                tracing::debug!(input_tokens, "message_start received");
            }

            // A new content block is starting.
            StreamEvent::ContentBlockStart(cbs) => {
                if let OutputContentBlock::ToolUse { id, name, .. } =
                    cbs.content_block
                {
                    tracing::debug!(tool_name = %name, tool_id = %id, "tool_use block started");
                    tracing::info!(tool_name = %name, "tool_use_detected");
                    pending_tools.insert(
                        cbs.index,
                        PendingTool {
                            id,
                            name,
                            input_buffer: String::new(),
                        },
                    );
                }
            }

            // Incremental content within a block.
            StreamEvent::ContentBlockDelta(cbd) => match cbd.delta {
                ContentBlockDelta::TextDelta { text } => {
                    events.push(AssistantEvent::TextDelta(text));
                }
                ContentBlockDelta::InputJsonDelta { partial_json } => {
                    if let Some(tool) = pending_tools.get_mut(&cbd.index) {
                        tool.input_buffer.push_str(&partial_json);
                    }
                }
                // Thinking / signature deltas are not surfaced.
                ContentBlockDelta::ThinkingDelta { .. }
                | ContentBlockDelta::SignatureDelta { .. } => {}
            },

            // A content block finished – flush any pending tool.
            StreamEvent::ContentBlockStop(cbs) => {
                if let Some(tool) = pending_tools.remove(&cbs.index) {
                    tracing::debug!(
                        tool_name = %tool.name,
                        input_len = tool.input_buffer.len(),
                        "tool_use block completed"
                    );
                    events.push(AssistantEvent::ToolUse {
                        id: tool.id,
                        name: tool.name,
                        input: tool.input_buffer,
                    });
                }
            }

            // Final usage snapshot from the model.
            StreamEvent::MessageDelta(md) => {
                let output_tokens = md.usage.output_tokens;
                tracing::debug!(output_tokens, "message_delta with usage");
                events.push(AssistantEvent::Usage(TokenUsage {
                    input_tokens,
                    output_tokens,
                    cache_creation_input_tokens,
                    cache_read_input_tokens,
                }));
            }

            StreamEvent::MessageStop(_) => {
                tracing::debug!(total_events = event_count, "message_stop received");
                events.push(AssistantEvent::MessageStop);
            }
        }
    }

    Ok(events)
}

// ---------------------------------------------------------------------------
// Message translation: runtime → api
// ---------------------------------------------------------------------------

fn translate_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter(|msg| msg.role != MessageRole::System)
        .map(|msg| {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "user",
                MessageRole::System => unreachable!("filtered above"),
            };

            let content = msg
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text {
                        text: text.clone(),
                    },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or(Value::String(input.clone())),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect();

            InputMessage {
                role: role.to_string(),
                content,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tool definitions for the SmartTrade tool executor
// ---------------------------------------------------------------------------

fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "classify_intent".to_string(),
            description: Some(
                "Classify the user's message intent (strategy_creation, strategy_refinement, \
                 clarification_response, explanation_request, or general)."
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_message": {
                        "type": "string",
                        "description": "The user message to classify"
                    }
                },
                "required": ["user_message"]
            }),
        },
        ToolDefinition {
            name: "detect_ambiguity".to_string(),
            description: Some(
                "Check whether the strategy specification has enough detail to proceed with \
                 code generation. Returns which fields are missing and a follow-up question."
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Current session ID" },
                    "action": { "type": "string", "description": "What the strategy should do" },
                    "pair": { "type": "string", "description": "Trading pair, e.g. EURUSD" },
                    "entry_condition": { "type": "string", "description": "When to enter a trade" },
                    "exit_condition": { "type": "string", "description": "When to exit a trade" },
                    "stop_loss": { "type": "string", "description": "Stop-loss rule" },
                    "timeframe": { "type": "string", "description": "Chart timeframe, e.g. H1" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "search_knowledge_base".to_string(),
            description: Some(
                "Search the MQL5 documentation and template knowledge base for relevant \
                 code examples and reference material."
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "top_k": {
                        "type": "integer",
                        "description": "Number of results to return (default 5)"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "run_static_analysis".to_string(),
            description: Some(
                "Run static analysis checks on MQL5 source code. Reports issues \
                 such as missing stoploss, unused variables, and anti-patterns."
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "MQL5 source code to analyse" },
                    "session_id": { "type": "string", "description": "Current session ID" }
                },
                "required": ["code"]
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::tool_definitions;

    #[test]
    fn server_owned_tools_are_not_exposed_to_llm() {
        let tool_names = tool_definitions()
            .into_iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>();

        assert!(!tool_names.iter().any(|name| name == "compile_mql5"));
        assert!(!tool_names.iter().any(|name| name == "save_strategy"));
    }
}
