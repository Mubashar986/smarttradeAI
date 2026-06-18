use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::postgres::PgPoolOptions;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime as TokioRuntime};
use uuid::Uuid;

use crate::conversation::{ToolError, ToolExecutor};

type JsonObject = serde_json::Map<String, JsonValue>;

pub const MAX_CLARIFICATION_ROUNDS: u64 = 5;
pub const MAX_STATIC_RETRIES: u64 = 3;
pub const MAX_COMPILE_RETRIES: u64 = 2;

const STRATEGY_CREATION_PATTERNS: &[&str] = &[
    r"(?i)\b(create|build|make|generate|write|code|develop)\b.*\b(ea|expert\s*advisor|strategy|bot|robot|algo)\b",
    r"(?i)\b(ea|expert\s*advisor|strategy|bot)\b.*\b(for|that|which|to)\b",
    r"(?i)\b(buy|sell|long|short)\b.*\b(when|if|once)\b",
    r"(?i)\b(sma|ema|rsi|macd|bollinger|stochastic|ichimoku)\b.*\b(cross|above|below|signal)\b",
    r"(?i)\b(scalp|swing|day\s*trad|grid|martingale|hedge)\b",
];

const STRATEGY_REFINEMENT_PATTERNS: &[&str] = &[
    r"(?i)\b(modify|change|update|adjust|improve|optimize|add|remove|tweak)\b.*\b(strategy|ea|code|logic|parameter)\b",
    r"(?i)\b(instead of|rather than|change.*to|replace.*with)\b",
    r"(?i)\b(add|include)\b.*\b(trailing\s*stop|take\s*profit|filter|indicator)\b",
];

const CLARIFICATION_RESPONSE_PATTERNS: &[&str] = &[
    r"(?i)^\s*\d+\s*(pip|point|lot|percent|%)",
    r"(?i)^\s*(h1|h4|m1|m5|m15|m30|d1|w1|mn)\s*$",
    r"(?i)^\s*(eurusd|gbpusd|usdjpy|audusd|usdcad|nzdusd|usdchf|xauusd|btcusd)\s*$",
    r"(?i)^\s*(yes|no|correct|exactly|right|that'?s?\s*right)\s*$",
    r"(?i)^\s*(buy|sell|long|short)\s*$",
    r"(?i)^\s*\d+\s*$",
    r"(?i)\b(reverse\s*cross|opposite\s*signal|close\s*on)\b",
];

const EXPLANATION_REQUEST_PATTERNS: &[&str] = &[
    r"(?i)\b(explain|what\s*(is|does|are)|how\s*(does|do|to)|why|tell\s*me\s*about|describe)\b",
    r"(?i)\b(what\s*happened|show\s*me|walk\s*me\s*through)\b",
    r"(?i)\b(difference\s*between|compare)\b",
];

const REQUIRED_FIELDS: [(&str, &str); 6] = [
    ("action", "What trading action? (BUY or SELL)"),
    (
        "pair",
        "Which trading pair? (e.g., EURUSD, GBPUSD, XAUUSD)",
    ),
    (
        "entry_condition",
        "What is the entry condition? (e.g., 'when 50 SMA crosses above 200 SMA')",
    ),
    (
        "exit_condition",
        "What is the exit condition? (e.g., 'reverse cross' or 'take profit at 100 pips')",
    ),
    (
        "stop_loss",
        "What stop-loss do you want? (e.g., '50 pips' or '1% of balance')",
    ),
    ("timeframe", "What chart timeframe? (e.g., M15, H1, H4, D1)"),
];


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StrategyIntent {
    StrategyCreation,
    StrategyRefinement,
    ClarificationResponse,
    ExplanationRequest,
    General,
}

impl StrategyIntent {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StrategyCreation => "STRATEGY_CREATION",
            Self::StrategyRefinement => "STRATEGY_REFINEMENT",
            Self::ClarificationResponse => "CLARIFICATION_RESPONSE",
            Self::ExplanationRequest => "EXPLANATION_REQUEST",
            Self::General => "GENERAL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentClassification {
    pub intent: StrategyIntent,
    pub confidence: f32,
    pub all_scores: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategySpec {
    pub action: Option<String>,
    pub pair: Option<String>,
    pub entry_condition: Option<String>,
    pub exit_condition: Option<String>,
    pub stop_loss: Option<String>,
    pub timeframe: Option<String>,
}

impl StrategySpec {
    #[must_use]
    pub fn provided_fields(&self) -> Vec<String> {
        REQUIRED_FIELDS
            .iter()
            .filter_map(|(field, _)| self.value_for(field).map(|_| (*field).to_string()))
            .collect()
    }

    #[must_use]
    pub fn as_map(&self) -> BTreeMap<String, String> {
        REQUIRED_FIELDS
            .iter()
            .filter_map(|(field, _)| {
                self.value_for(field)
                    .map(|value| ((*field).to_string(), value))
            })
            .collect()
    }

    #[must_use]
    pub fn value_for(&self, field: &str) -> Option<String> {
        match field {
            "action" => self.action.clone(),
            "pair" => self.pair.clone(),
            "entry_condition" => self.entry_condition.clone(),
            "exit_condition" => self.exit_condition.clone(),
            "stop_loss" => self.stop_loss.clone(),
            "timeframe" => self.timeframe.clone(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AmbiguityStatus {
    Incomplete,
    Complete,
    DraftSaved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AmbiguityResult {
    pub status: AmbiguityStatus,
    pub round: u64,
    pub max_rounds: u64,
    pub message: String,
    pub spec: BTreeMap<String, String>,
    pub missing_fields: Vec<String>,
    pub missing_count: usize,
    pub provided_fields: Vec<String>,
    pub next_question: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedStrategy {
    pub strategy_name: String,
    pub code: String,
    pub explanation: String,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticAnalysisIssue {
    #[serde(rename = "type")]
    pub issue_type: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticAnalysisResult {
    pub passed: bool,
    pub status: String,
    pub retry: u64,
    pub max_retries: u64,
    pub error_count: usize,
    pub warning_count: usize,
    pub message: String,
    pub errors: Vec<StaticAnalysisIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeBaseMatch {
    pub id: String,
    pub score: f32,
    pub content: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeBaseSearchResult {
    pub source: String,
    pub results: Vec<KnowledgeBaseMatch>,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompilerMessage {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub retry: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u64>,
    pub errors: Vec<CompilerMessage>,
    pub warnings: Vec<CompilerMessage>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveStrategyRequest {
    pub strategy_name: String,
    pub code: String,
    pub explanation: String,
    pub status: String,
    pub session_id: String,
    pub user_id: String,
    pub pair: String,
    pub timeframe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveStrategyResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub storage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartTradeToolConfig {
    pub compiler_url: Option<String>,
    pub database_url: Option<String>,
    pub pinecone_api_key: Option<String>,
    pub pinecone_index: String,
    pub pinecone_namespace: String,
    pub strategies_dir: PathBuf,
}

impl SmartTradeToolConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            compiler_url: non_empty_env("C3_COMPILER_URL"),
            database_url: non_empty_env("DATABASE_URL"),
            pinecone_api_key: non_empty_env("PINECONE_API_KEY"),
            pinecone_index: std::env::var("PINECONE_INDEX")
                .unwrap_or_else(|_| "mql5-docs".to_string()),
            pinecone_namespace: std::env::var("PINECONE_NAMESPACE")
                .unwrap_or_else(|_| "mql5_templates".to_string()),
            strategies_dir: std::env::var("STRATEGIES_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/workspace/strategies")),
        }
    }
}

impl Default for SmartTradeToolConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SessionToolState {
    ambiguity_round: u64,
    static_retry: u64,
    compile_retry: u64,
    static_passed: bool,
    compile_passed: bool,
}

#[derive(Debug, Default)]
pub struct SmartTradeToolExecutor {
    config: SmartTradeToolConfig,
    session_state: BTreeMap<String, SessionToolState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationError {
    message: String,
}

impl GenerationError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for GenerationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for GenerationError {}

impl SmartTradeToolExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_config(config: SmartTradeToolConfig) -> Self {
        Self {
            config,
            session_state: BTreeMap::new(),
        }
    }

    fn session_state_mut(&mut self, session_id: &str) -> &mut SessionToolState {
        self.session_state
            .entry(session_id.to_string())
            .or_default()
    }

    fn detect_ambiguity_tool(&mut self, payload: &JsonValue) -> Result<String, ToolError> {
        let session_id = payload
            .get("session_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        let round = if session_id == "unknown" {
            1
        } else {
            let state = self.session_state_mut(session_id);
            state.ambiguity_round += 1;
            state.ambiguity_round
        };
        let spec = strategy_spec_from_value(payload);
        let result = detect_ambiguity(&spec, round);
        if result.status == AmbiguityStatus::Complete && session_id != "unknown" {
            self.session_state_mut(session_id).ambiguity_round = 0;
        }
        to_tool_json(&result)
    }

    fn run_static_analysis_tool(&mut self, payload: &JsonValue) -> Result<String, ToolError> {
        let code = required_string_field(payload, "code")?;
        let session_id = payload
            .get("session_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        let retry = if session_id == "unknown" {
            1
        } else {
            let state = self.session_state_mut(session_id);
            state.static_retry += 1;
            state.static_retry
        };
        let result = run_static_analysis(&code, retry);
        if session_id != "unknown" {
            let state = self.session_state_mut(session_id);
            state.static_passed = result.passed;
            if result.passed {
                state.static_retry = 0;
            }
        }
        to_tool_json(&result)
    }

    fn compile_mql5_tool(&mut self, payload: &JsonValue) -> Result<String, ToolError> {
        let code = required_string_field(payload, "code")?;
        let session_id = payload
            .get("session_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        let retry = if session_id == "unknown" {
            1
        } else {
            let state = self.session_state_mut(session_id);
            state.compile_retry += 1;
            state.compile_retry
        };
        let result = compile_mql5_with_config(&code, session_id, retry, &self.config);
        if session_id != "unknown" {
            let state = self.session_state_mut(session_id);
            state.compile_passed = result.success;
            if result.success {
                state.compile_retry = 0;
            }
        }
        to_tool_json(&result)
    }
}

impl ToolExecutor for SmartTradeToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        match tool_name {
            "classify_intent" => {
                let payload = parse_optional_json(input);
                let message = payload
                    .as_ref()
                    .and_then(|value| value.get("user_message").and_then(JsonValue::as_str))
                    .or_else(|| {
                        payload
                            .as_ref()
                            .and_then(|value| value.get("message").and_then(JsonValue::as_str))
                    })
                    .unwrap_or(input);
                to_tool_json(&classify_intent(message))
            }
            "detect_ambiguity" => {
                let payload = parse_required_json(input)?;
                self.detect_ambiguity_tool(&payload)
            }
            "search_knowledge_base" => {
                let payload = parse_required_json(input)?;
                let query = required_string_field(&payload, "query")?;
                let top_k = payload
                    .get("top_k")
                    .and_then(JsonValue::as_u64)
                    .unwrap_or(5) as usize;
                to_tool_json(&search_knowledge_base_with_config(
                    &query,
                    top_k,
                    &self.config,
                ))
            }

            "run_static_analysis" => {
                let payload = parse_required_json(input)?;
                self.run_static_analysis_tool(&payload)
            }
            "compile_mql5" => {
                let payload = parse_required_json(input)?;
                self.compile_mql5_tool(&payload)
            }
            "save_strategy" => {
                let payload = parse_required_json(input)?;
                let request = save_request_from_value(&payload)?;
                to_tool_json(&save_strategy_with_config(&request, &self.config))
            }
            other => Err(ToolError::new(format!("unknown tool: {other}"))),
        }
    }
}

#[must_use]
pub fn classify_intent(message: &str) -> IntentClassification {
    let text = message.trim();
    let mut scores = BTreeMap::new();

    record_score(
        &mut scores,
        StrategyIntent::StrategyCreation,
        STRATEGY_CREATION_PATTERNS,
        text,
    );
    record_score(
        &mut scores,
        StrategyIntent::StrategyRefinement,
        STRATEGY_REFINEMENT_PATTERNS,
        text,
    );
    record_score(
        &mut scores,
        StrategyIntent::ClarificationResponse,
        CLARIFICATION_RESPONSE_PATTERNS,
        text,
    );
    record_score(
        &mut scores,
        StrategyIntent::ExplanationRequest,
        EXPLANATION_REQUEST_PATTERNS,
        text,
    );

    let Some((best_intent, best_score)) = [
        (
            StrategyIntent::StrategyCreation,
            score_for(&scores, StrategyIntent::StrategyCreation),
        ),
        (
            StrategyIntent::StrategyRefinement,
            score_for(&scores, StrategyIntent::StrategyRefinement),
        ),
        (
            StrategyIntent::ClarificationResponse,
            score_for(&scores, StrategyIntent::ClarificationResponse),
        ),
        (
            StrategyIntent::ExplanationRequest,
            score_for(&scores, StrategyIntent::ExplanationRequest),
        ),
    ]
    .into_iter()
    .max_by_key(|(_, score)| *score)
    .filter(|(_, score)| *score > 0)
    else {
        return IntentClassification {
            intent: StrategyIntent::General,
            confidence: 0.5,
            all_scores: BTreeMap::new(),
        };
    };

    let max_possible = max_pattern_count(best_intent) as f32;
    let confidence = (best_score as f32 / max_possible).min(1.0);

    IntentClassification {
        intent: best_intent,
        confidence: (confidence * 100.0).round() / 100.0,
        all_scores: scores,
    }
}

#[must_use]
pub fn extract_strategy_spec(message: &str) -> StrategySpec {
    let normalized = normalize_whitespace(message);
    StrategySpec {
        action: extract_action(&normalized),
        pair: capture_upper(
            &normalized,
            &[r"(?i)\b(EURUSD|GBPUSD|USDJPY|AUDUSD|USDCAD|NZDUSD|USDCHF|XAUUSD|BTCUSD)\b"],
        ),
        entry_condition: capture_preserving_case(
            &normalized,
            &[
                r"(?i)\bwhen\s+(?P<value>[^.!?\n]+)",
                r"(?i)\bif\s+(?P<value>[^.!?\n]+)",
            ],
        ),
        exit_condition: capture_preserving_case(
            &normalized,
            &[
                r"(?i)\b(reverse cross|opposite signal)\b",
                r"(?i)\b(close on [^.!?\n]+)",
                r"(?i)\b(exit(?:s|ing)?(?:\s+when)?\s+[^.!?\n]+)",
                r"(?i)\b(take\s*profit[^.!?\n]+)",
            ],
        ),
        stop_loss: capture_preserving_case(
            &normalized,
            &[
                r"(?i)\bstop[- ]?loss\b(?:\s*(?:at|of|is|=|:))?\s*(?P<value>\d+\s*(?:pip|pips|point|points|percent|%))",
                r"(?i)\bsl\b(?:\s*(?:at|of|is|=|:))?\s*(?P<value>\d+\s*(?:pip|pips|point|points|percent|%))",
            ],
        ),
        timeframe: capture_upper(&normalized, &[r"(?i)\b(M1|M5|M15|M30|H1|H4|D1|W1|MN)\b"]),
    }
}

#[must_use]
pub fn detect_ambiguity(spec: &StrategySpec, round: u64) -> AmbiguityResult {
    if round > MAX_CLARIFICATION_ROUNDS {
        return AmbiguityResult {
            status: AmbiguityStatus::DraftSaved,
            round,
            max_rounds: MAX_CLARIFICATION_ROUNDS,
            message: format!(
                "Maximum clarification rounds ({MAX_CLARIFICATION_ROUNDS}) exceeded. Saving current spec as DRAFT."
            ),
            spec: spec.as_map(),
            missing_fields: Vec::new(),
            missing_count: 0,
            provided_fields: spec.provided_fields(),
            next_question: None,
        };
    }

    let mut missing_fields = Vec::new();
    let mut next_question = None;
    for (field, question) in REQUIRED_FIELDS {
        if spec.value_for(field).is_none() {
            missing_fields.push(field.to_string());
            if next_question.is_none() {
                next_question = Some(question.to_string());
            }
        }
    }

    if missing_fields.is_empty() {
        return AmbiguityResult {
            status: AmbiguityStatus::Complete,
            round,
            max_rounds: MAX_CLARIFICATION_ROUNDS,
            message: "All required parameters are present. Ready for code generation.".to_string(),
            spec: spec.as_map(),
            missing_fields,
            missing_count: 0,
            provided_fields: spec.provided_fields(),
            next_question: None,
        };
    }

    AmbiguityResult {
        status: AmbiguityStatus::Incomplete,
        round,
        max_rounds: MAX_CLARIFICATION_ROUNDS,
        message: "More strategy details are required before code generation.".to_string(),
        spec: spec.as_map(),
        missing_count: missing_fields.len(),
        provided_fields: spec.provided_fields(),
        missing_fields,
        next_question,
    }
}

#[must_use]
pub fn search_knowledge_base(query: &str, top_k: usize) -> KnowledgeBaseSearchResult {
    search_knowledge_base_with_config(query, top_k, &SmartTradeToolConfig::from_env())
}


#[must_use]
pub fn run_static_analysis(code: &str, retry: u64) -> StaticAnalysisResult {
    if retry > MAX_STATIC_RETRIES {
        return StaticAnalysisResult {
            passed: false,
            status: "MAX_RETRIES_EXCEEDED".to_string(),
            retry,
            max_retries: MAX_STATIC_RETRIES,
            error_count: 0,
            warning_count: 0,
            message: format!(
                "Static analysis failed after {MAX_STATIC_RETRIES} attempts. Saving as FAILED."
            ),
            errors: Vec::new(),
        };
    }

    let mut errors = Vec::new();

    let open_braces = code.matches('{').count();
    let close_braces = code.matches('}').count();
    if open_braces != close_braces {
        errors.push(issue(
            "BRACKET_MISMATCH",
            "ERROR",
            format!("Unbalanced braces: {open_braces} open, {close_braces} close"),
        ));
    }

    let open_parens = code.matches('(').count();
    let close_parens = code.matches(')').count();
    if open_parens != close_parens {
        errors.push(issue(
            "PAREN_MISMATCH",
            "ERROR",
            format!("Unbalanced parentheses: {open_parens} open, {close_parens} close"),
        ));
    }

    for function_name in ["OnInit", "OnDeinit", "OnTick"] {
        if !Regex::new(&format!(r"\b{function_name}\s*\("))
            .is_ok_and(|regex| regex.is_match(code))
        {
            errors.push(issue(
                "MISSING_FUNCTION",
                "ERROR",
                format!("Required function {function_name}() not found"),
            ));
        }
    }

    if let Some(captures) = Regex::new(r"(\w+)\s+OnInit\s*\(")
        .ok()
        .and_then(|regex| regex.captures(code))
    {
        if captures.get(1).is_some_and(|group| group.as_str() != "int") {
            errors.push(issue(
                "WRONG_SIGNATURE",
                "ERROR",
                format!(
                    "OnInit() must return int, found {}",
                    captures
                        .get(1)
                        .map_or("unknown", |group| group.as_str())
                ),
            ));
        }
    }

    let lower = code.to_lowercase();
    let has_order_send =
        code.contains("OrderSend") || code.contains("CTrade") || lower.contains("trade.");
    let has_stop_loss = [
        "stoploss",
        "stop_loss",
        "stoplosspips",
        "request.sl",
        ".sl(",
        ".sl =",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if has_order_send && !has_stop_loss {
        errors.push(issue(
            "MISSING_STOPLOSS",
            "ERROR",
            "Order placement detected without stop-loss assignment. Every trade MUST have a stop-loss.",
        ));
    }

    for (deprecated, suggestion) in [
        (
            "OrderSend(Symbol()",
            "Use CTrade class or MQL5-style OrderSend with MqlTradeRequest",
        ),
        ("OrderClose(", "Use CTrade.PositionClose() in MQL5"),
        ("OrderModify(", "Use CTrade.PositionModify() in MQL5"),
        (
            "OrderSelect(",
            "Use PositionSelect() or PositionGetTicket() in MQL5",
        ),
        (
            "OrdersTotal()",
            "Use PositionsTotal() for open positions in MQL5",
        ),
    ] {
        if code.contains(deprecated) {
            errors.push(issue(
                "DEPRECATED_FUNCTION",
                "WARNING",
                format!("Deprecated MQL4 function detected: {deprecated}. {suggestion}"),
            ));
        }
    }

    if !code.contains("#property strict") && !code.contains("#property version") {
        errors.push(issue(
            "MISSING_PROPERTY",
            "WARNING",
            "Missing #property directive. Add #property version or #property strict.",
        ));
    }

    let error_count = errors
        .iter()
        .filter(|issue| issue.severity == "ERROR")
        .count();
    let warning_count = errors.len().saturating_sub(error_count);
    let passed = error_count == 0;

    StaticAnalysisResult {
        passed,
        status: if passed {
            "PASSED".to_string()
        } else {
            "FAILED".to_string()
        },
        retry,
        max_retries: MAX_STATIC_RETRIES,
        error_count,
        warning_count,
        message: if passed {
            "Static analysis passed.".to_string()
        } else {
            "Static analysis found issues that need another correction loop.".to_string()
        },
        errors,
    }
}

#[must_use]
pub fn compile_mql5(code: &str, session_id: &str, retry: u64) -> CompileResult {
    compile_mql5_with_config(code, session_id, retry, &SmartTradeToolConfig::from_env())
}

#[must_use]
pub fn save_strategy(request: &SaveStrategyRequest) -> SaveStrategyResult {
    save_strategy_with_config(request, &SmartTradeToolConfig::from_env())
}

fn search_knowledge_base_with_config(
    _query: &str,
    _top_k: usize,
    config: &SmartTradeToolConfig,
) -> KnowledgeBaseSearchResult {
    KnowledgeBaseSearchResult {
        source: "local_fallback".to_string(),
        count: 0,
        results: Vec::new(),
        note: if config.pinecone_api_key.is_some() {
            Some(format!(
                "Pinecone is configured for index `{}` / namespace `{}`, but local search is disabled and Pinecone query is not fully implemented.",
                config.pinecone_index, config.pinecone_namespace
            ))
        } else {
            Some("Pinecone not configured - knowledge base is empty".to_string())
        },
    }
}

fn compile_mql5_with_config(
    code: &str,
    session_id: &str,
    retry: u64,
    config: &SmartTradeToolConfig,
) -> CompileResult {
    if retry > MAX_COMPILE_RETRIES {
        return CompileResult {
            success: false,
            status: Some("MAX_RETRIES_EXCEEDED".to_string()),
            retry,
            max_retries: Some(MAX_COMPILE_RETRIES),
            errors: Vec::new(),
            warnings: Vec::new(),
            source: "max_retries".to_string(),
            note: None,
            message: Some(format!(
                "Compilation failed after {MAX_COMPILE_RETRIES} attempts."
            )),
        };
    }

    let Some(compiler_url) = &config.compiler_url else {
        return CompileResult {
            success: true,
            status: None,
            retry,
            max_retries: Some(MAX_COMPILE_RETRIES),
            errors: Vec::new(),
            warnings: vec![CompilerMessage {
                message:
                    "C3 Quality Lab not configured - compilation check skipped (STUB MODE)"
                        .to_string(),
            }],
            source: "stub".to_string(),
            note: Some(
                "Set C3_COMPILER_URL environment variable to enable real MetaEditor compilation"
                    .to_string(),
            ),
            message: None,
        };
    };

    let compiler_url = compiler_url.clone();
    let payload = json_value_object([
        ("code", JsonValue::String(code.to_string())),
        ("session_id", JsonValue::String(session_id.to_string())),
    ]);

    match tool_block_on(async move {
        let response = reqwest::Client::new()
            .post(&compiler_url)
            .json(&payload)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        response
            .json::<JsonValue>()
            .await
            .map_err(|error| error.to_string())
    }) {
        Ok(Ok(body)) => compile_result_from_json(body, retry),
        Ok(Err(error)) => CompileResult {
            success: false,
            status: None,
            retry,
            max_retries: Some(MAX_COMPILE_RETRIES),
            errors: vec![CompilerMessage {
                message: format!("C3 compiler returned invalid JSON: {error}"),
            }],
            warnings: Vec::new(),
            source: "c3_error".to_string(),
            note: None,
            message: None,
        },
        Err(error) => CompileResult {
            success: false,
            status: None,
            retry,
            max_retries: Some(MAX_COMPILE_RETRIES),
            errors: vec![CompilerMessage {
                message: format!("C3 compiler unreachable: {error}"),
            }],
            warnings: Vec::new(),
            source: "c3_error".to_string(),
            note: None,
            message: None,
        },
    }
}

fn save_strategy_with_config(
    request: &SaveStrategyRequest,
    config: &SmartTradeToolConfig,
) -> SaveStrategyResult {
    match &config.database_url {
        Some(database_url) if !database_url.is_empty() => {
            match persist_strategy_postgres(request, database_url) {
                Ok(strategy_id) => SaveStrategyResult {
                    success: true,
                    strategy_id: Some(strategy_id),
                    status: Some(request.status.clone()),
                    storage: "postgresql".to_string(),
                    file_path: None,
                    error: None,
                },
                Err(error) => SaveStrategyResult {
                    success: false,
                    strategy_id: None,
                    status: None,
                    storage: "postgresql_error".to_string(),
                    file_path: None,
                    error: Some(error),
                },
            }
        }
        _ => persist_strategy_local(request, &config.strategies_dir),
    }
}

fn persist_strategy_postgres(
    request: &SaveStrategyRequest,
    database_url: &str,
) -> Result<String, String> {
    let database_url = database_url.to_string();
    let request = request.clone();
    match tool_block_on(async move {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .map_err(|error| error.to_string())?;
        let strategy_id: i32 = sqlx::query_scalar(
            r#"
            INSERT INTO strategies
                (name, code, explanation, status, session_id, user_id, pair, timeframe, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
            RETURNING id
            "#,
        )
        .bind(&request.strategy_name)
        .bind(&request.code)
        .bind(&request.explanation)
        .bind(&request.status)
        .bind(&request.session_id)
        .bind(&request.user_id)
        .bind(&request.pair)
        .bind(&request.timeframe)
        .fetch_one(&pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok::<String, String>(strategy_id.to_string())
    })
    {
        Ok(result) => result,
        Err(error) => Err(error),
    }
}

fn persist_strategy_local(
    request: &SaveStrategyRequest,
    strategies_dir: &Path,
) -> SaveStrategyResult {
    if let Err(error) = fs::create_dir_all(strategies_dir) {
        return SaveStrategyResult {
            success: false,
            strategy_id: None,
            status: None,
            storage: "local_file_error".to_string(),
            file_path: None,
            error: Some(error.to_string()),
        };
    }

    let strategy_id = format!("local-{}", Uuid::new_v4());
    let timestamp = current_timestamp_slug();
    let sanitized_name = sanitize_file_stem(&request.strategy_name);
    let file_path = strategies_dir.join(format!("{sanitized_name}_{timestamp}.mq5"));
    if let Err(error) = fs::write(&file_path, &request.code) {
        return SaveStrategyResult {
            success: false,
            strategy_id: None,
            status: None,
            storage: "local_file_error".to_string(),
            file_path: Some(file_path.display().to_string()),
            error: Some(error.to_string()),
        };
    }

    let metadata_path = file_path.with_extension("json");
    let metadata = json_value_object([
        ("strategy_id", JsonValue::String(strategy_id.clone())),
        (
            "strategy_name",
            JsonValue::String(request.strategy_name.clone()),
        ),
        ("status", JsonValue::String(request.status.clone())),
        (
            "session_id",
            JsonValue::String(request.session_id.clone()),
        ),
        ("user_id", JsonValue::String(request.user_id.clone())),
        ("pair", JsonValue::String(request.pair.clone())),
        (
            "timeframe",
            JsonValue::String(request.timeframe.clone()),
        ),
        (
            "explanation",
            JsonValue::String(request.explanation.clone()),
        ),
        (
            "created_at",
            JsonValue::String(current_iso8601_like_timestamp()),
        ),
        (
            "updated_at",
            JsonValue::String(current_iso8601_like_timestamp()),
        ),
    ]);

    if let Err(error) = fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|serialization_error| format!("{{\"error\":\"{serialization_error}\"}}")),
    ) {
        return SaveStrategyResult {
            success: false,
            strategy_id: None,
            status: None,
            storage: "local_file_error".to_string(),
            file_path: Some(file_path.display().to_string()),
            error: Some(error.to_string()),
        };
    }

    SaveStrategyResult {
        success: true,
        strategy_id: Some(strategy_id),
        status: Some(request.status.clone()),
        storage: "local_file".to_string(),
        file_path: Some(file_path.display().to_string()),
        error: None,
    }
}

fn compile_result_from_json(body: JsonValue, retry: u64) -> CompileResult {
    let success = body
        .get("success")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    CompileResult {
        success,
        status: body
            .get("status")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
        retry,
        max_retries: Some(MAX_COMPILE_RETRIES),
        errors: compiler_messages_from_value(body.get("errors")),
        warnings: compiler_messages_from_value(body.get("warnings")),
        source: body
            .get("source")
            .and_then(JsonValue::as_str)
            .unwrap_or("c3_metaeditor")
            .to_string(),
        note: body
            .get("note")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
        message: body
            .get("message")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
    }
}

fn compiler_messages_from_value(value: Option<&JsonValue>) -> Vec<CompilerMessage> {
    value
        .and_then(JsonValue::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    if let Some(message) = entry.get("message").and_then(JsonValue::as_str) {
                        Some(CompilerMessage {
                            message: message.to_string(),
                        })
                    } else {
                        entry.as_str().map(|message| CompilerMessage {
                            message: message.to_string(),
                        })
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_required_json(input: &str) -> Result<JsonValue, ToolError> {
    serde_json::from_str(input)
        .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))
}

fn parse_optional_json(input: &str) -> Option<JsonValue> {
    serde_json::from_str(input).ok()
}

fn to_tool_json<T: Serialize>(value: &T) -> Result<String, ToolError> {
    serde_json::to_string(value)
        .map_err(|error| ToolError::new(format!("failed to serialize tool output: {error}")))
}

fn required_string_field(payload: &JsonValue, field: &str) -> Result<String, ToolError> {
    payload
        .get(field)
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| ToolError::new(format!("missing string field `{field}`")))
}

fn strategy_spec_from_value(payload: &JsonValue) -> StrategySpec {
    StrategySpec {
        action: payload.get("action").and_then(JsonValue::as_str).map(ToOwned::to_owned),
        pair: payload.get("pair").and_then(JsonValue::as_str).map(ToOwned::to_owned),
        entry_condition: payload
            .get("entry_condition")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
        exit_condition: payload
            .get("exit_condition")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
        stop_loss: payload
            .get("stop_loss")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
        timeframe: payload
            .get("timeframe")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned),
    }
}

fn save_request_from_value(payload: &JsonValue) -> Result<SaveStrategyRequest, ToolError> {
    Ok(SaveStrategyRequest {
        strategy_name: payload
            .get("strategy_name")
            .and_then(JsonValue::as_str)
            .unwrap_or("Unnamed")
            .to_string(),
        code: required_string_field(payload, "code")?,
        explanation: payload
            .get("explanation")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        status: payload
            .get("status")
            .and_then(JsonValue::as_str)
            .unwrap_or("DRAFT")
            .to_string(),
        session_id: payload
            .get("session_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        user_id: payload
            .get("user_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        pair: payload
            .get("pair")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        timeframe: payload
            .get("timeframe")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

fn parameter_lines_from_json(parameters: Option<&JsonObject>) -> String {
    let mut lines = Vec::new();
    if let Some(parameters) = parameters {
        for (name, value) in parameters {
            match value {
                JsonValue::Number(number) if number.is_i64() || number.is_u64() => {
                    lines.push(format!("input int {name} = {number};"));
                }
                JsonValue::Number(number) => {
                    lines.push(format!("input double {name} = {number};"));
                }
                JsonValue::String(text) => {
                    lines.push(format!(
                        "input string {name} = \"{}\";",
                        escape_mql5_string(&text)
                    ));
                }
                _ => {}
            }
        }
    }
    if lines.is_empty() {
        "// No custom parameters".to_string()
    } else {
        lines.join("\n")
    }
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    let mut truncated = content.chars().take(max_chars).collect::<String>();
    if content.chars().count() > max_chars {
        truncated.push_str("...");
    }
    truncated
}

fn sanitize_file_stem(name: &str) -> String {
    let fallback = "Unnamed";
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    trimmed
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn current_timestamp_slug() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn current_iso8601_like_timestamp() -> String {
    format!("unix:{}", current_timestamp_slug())
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.trim().is_empty())
}

fn json_value_object<const N: usize>(entries: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<JsonObject>(),
    )
}

fn tool_runtime() -> Result<TokioRuntime, String> {
    RuntimeBuilder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())
}

fn tool_block_on<F>(future: F) -> Result<F::Output, String>
where
    F: Future,
{
    tool_runtime().map(|runtime| runtime.block_on(future))
}

fn record_score(
    scores: &mut BTreeMap<String, u32>,
    intent: StrategyIntent,
    patterns: &[&str],
    text: &str,
) {
    let score = patterns
        .iter()
        .filter(|pattern| Regex::new(pattern).is_ok_and(|regex| regex.is_match(text)))
        .count() as u32;
    if score > 0 {
        scores.insert(intent.as_str().to_string(), score);
    }
}

fn score_for(scores: &BTreeMap<String, u32>, intent: StrategyIntent) -> u32 {
    scores.get(intent.as_str()).copied().unwrap_or_default()
}

const fn max_pattern_count(intent: StrategyIntent) -> usize {
    match intent {
        StrategyIntent::StrategyCreation => STRATEGY_CREATION_PATTERNS.len(),
        StrategyIntent::StrategyRefinement => STRATEGY_REFINEMENT_PATTERNS.len(),
        StrategyIntent::ClarificationResponse => CLARIFICATION_RESPONSE_PATTERNS.len(),
        StrategyIntent::ExplanationRequest => EXPLANATION_REQUEST_PATTERNS.len(),
        StrategyIntent::General => 1,
    }
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_action(text: &str) -> Option<String> {
    if Regex::new(r"(?i)\b(buy|long)\b").is_ok_and(|regex| regex.is_match(text)) {
        return Some("BUY".to_string());
    }
    if Regex::new(r"(?i)\b(sell|short)\b").is_ok_and(|regex| regex.is_match(text)) {
        return Some("SELL".to_string());
    }
    None
}

fn capture_upper(text: &str, patterns: &[&str]) -> Option<String> {
    capture_preserving_case(text, patterns).map(|value| value.to_uppercase())
}

fn capture_preserving_case(text: &str, patterns: &[&str]) -> Option<String> {
    for pattern in patterns {
        let Ok(regex) = Regex::new(pattern) else {
            continue;
        };
        if let Some(captures) = regex.captures(text) {
            if let Some(value) = captures.name("value") {
                return Some(value.as_str().trim().to_string());
            }
            if let Some(full_match) = captures.get(1).or_else(|| captures.get(0)) {
                return Some(full_match.as_str().trim().to_string());
            }
        }
    }
    None
}

fn sanitize_comment(value: &str) -> String {
    value.replace('\n', " ").replace('\r', " ").trim().to_string()
}

fn escape_mql5_string(value: &str) -> String {
    value.replace('"', "\\\"")
}

fn issue(
    issue_type: impl Into<String>,
    severity: impl Into<String>,
    message: impl Into<String>,
) -> StaticAnalysisIssue {
    StaticAnalysisIssue {
        issue_type: issue_type.into(),
        severity: severity.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_intent, compile_mql5, detect_ambiguity, extract_strategy_spec,
        save_strategy, search_knowledge_base, run_static_analysis,
        AmbiguityStatus, SaveStrategyRequest, SmartTradeToolExecutor, StrategyIntent,
    };
    use crate::conversation::ToolExecutor;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("smarttrade-tools-{nanos}"))
    }

    #[test]
    fn classifies_strategy_creation_requests() {
        let result = classify_intent(
            "Create an SMA crossover EA for EURUSD that buys when the 50 SMA crosses above the 200 SMA.",
        );

        assert_eq!(result.intent, StrategyIntent::StrategyCreation);
        assert!(result.confidence > 0.0);
        assert!(result
            .all_scores
            .contains_key(StrategyIntent::StrategyCreation.as_str()));
    }

    #[test]
    fn classifies_short_clarification_responses() {
        let result = classify_intent("H1");

        assert_eq!(result.intent, StrategyIntent::ClarificationResponse);
    }

    #[test]
    fn extracts_partial_strategy_spec_from_free_text() {
        let spec = extract_strategy_spec(
            "Create a simple SMA crossover strategy for EURUSD H1 with stop-loss 50 pips.",
        );

        assert_eq!(spec.pair.as_deref(), Some("EURUSD"));
        assert_eq!(spec.timeframe.as_deref(), Some("H1"));
        assert_eq!(spec.stop_loss.as_deref(), Some("50 pips"));
        assert!(spec.action.is_none());
        assert!(spec.exit_condition.is_none());
    }

    #[test]
    fn reports_missing_fields_one_at_a_time() {
        let spec = extract_strategy_spec(
            "Create a simple SMA crossover strategy for EURUSD H1 with stop-loss 50 pips.",
        );
        let result = detect_ambiguity(&spec, 1);

        assert_eq!(result.status, AmbiguityStatus::Incomplete);
        assert_eq!(
            result.next_question.as_deref(),
            Some("What trading action? (BUY or SELL)")
        );
        assert!(result.missing_fields.contains(&"action".to_string()));
        assert!(result.missing_fields.contains(&"entry_condition".to_string()));
    }

    #[test]
    fn marks_complete_specs_as_ready() {
        let spec = extract_strategy_spec(
            "Build a BUY EURUSD H1 strategy when 50 SMA crosses above 200 SMA with reverse cross exit and stop-loss 50 pips.",
        );
        let result = detect_ambiguity(&spec, 1);

        assert_eq!(result.status, AmbiguityStatus::Complete);
        assert_eq!(result.missing_count, 0);
        assert_eq!(result.spec.get("action").map(String::as_str), Some("BUY"));
    }

    #[test]
    fn falls_back_to_draft_after_too_many_rounds() {
        let spec = extract_strategy_spec("EURUSD H1");
        let result = detect_ambiguity(&spec, 6);

        assert_eq!(result.status, AmbiguityStatus::DraftSaved);
        assert!(result.message.contains("Maximum clarification rounds"));
    }


    #[test]
    fn static_analysis_detects_structural_problems() {
        let analysis = run_static_analysis(
            "void OnTick( { trade.Buy(0.1, _Symbol, 0, 0, 0, \"bad\"); }",
            1,
        );

        assert!(!analysis.passed);
        assert!(analysis
            .errors
            .iter()
            .any(|issue| issue.issue_type == "BRACKET_MISMATCH"));
        assert!(analysis
            .errors
            .iter()
            .any(|issue| issue.issue_type == "MISSING_FUNCTION"));
    }

    #[test]
    fn searches_local_knowledge_base_with_keyword_scoring() {
        let result = search_knowledge_base("sma crossover eurusd", 3);

        assert_eq!(result.source, "local_fallback");
        assert!(result.count >= 1);
        assert!(result
            .results
            .iter()
            .any(|entry| entry.id == "sma_crossover"));
    }

    #[test]
    fn compile_tool_uses_stub_mode_without_c3_url() {
        let _guard = crate::test_env_lock();
        std::env::remove_var("C3_COMPILER_URL");

        let result = compile_mql5("int OnInit(){return(INIT_SUCCEEDED);} void OnDeinit(const int reason){} void OnTick(){}", "session-1", 1);

        assert!(result.success);
        assert_eq!(result.source, "stub");
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.message.contains("STUB MODE")));
    }

    #[test]
    fn save_strategy_falls_back_to_local_files() {
        let _guard = crate::test_env_lock();
        let strategies_dir = temp_dir();
        std::env::set_var("STRATEGIES_DIR", &strategies_dir);
        std::env::remove_var("DATABASE_URL");

        let result = save_strategy(&SaveStrategyRequest {
            strategy_name: "Test Strategy".to_string(),
            code: "void OnTick() {}".to_string(),
            explanation: "example".to_string(),
            status: "GENERATED".to_string(),
            session_id: "session-1".to_string(),
            user_id: "user-1".to_string(),
            pair: "EURUSD".to_string(),
            timeframe: "H1".to_string(),
        });

        assert!(result.success);
        assert_eq!(result.storage, "local_file");
        let file_path = result
            .file_path
            .as_ref()
            .expect("local save should return file path");
        assert!(fs::metadata(file_path).is_ok());
        assert!(fs::metadata(file_path.replace(".mq5", ".json")).is_ok());

        fs::remove_dir_all(&strategies_dir).expect("cleanup temp dir");
        std::env::remove_var("STRATEGIES_DIR");
    }

    #[test]
    fn smarttrade_tool_executor_dispatches_known_tools() {
        let mut executor = SmartTradeToolExecutor::new();

        let classify = executor
            .execute(
                "classify_intent",
                r#"{"user_message":"Create a BUY EURUSD H1 strategy"}"#,
            )
            .expect("classify intent should succeed");
        let classify_json: serde_json::Value =
            serde_json::from_str(&classify).expect("classify output should be valid json");
        assert_eq!(classify_json["intent"], "STRATEGY_CREATION");

        let detect = executor
            .execute(
                "detect_ambiguity",
                r#"{"session_id":"session-1","pair":"EURUSD","timeframe":"H1"}"#,
            )
            .expect("detect ambiguity should succeed");
        let detect_json: serde_json::Value =
            serde_json::from_str(&detect).expect("detect output should be valid json");
        assert_eq!(detect_json["status"], "INCOMPLETE");
    }
}
