mod compact;
mod config;
mod conversation;
mod hooks;
mod oauth_support;
mod prompt;
mod session;
mod smarttrade_tools;
mod usage;

pub use compact::{
    compact_session, estimate_session_tokens, format_compact_summary,
    get_compact_continuation_message, should_compact, CompactionConfig, CompactionResult,
};
pub use config::{
    ConfigEntry, ConfigError, ConfigLoader, ConfigSource, OAuthConfig, ResolvedPermissionMode,
    RuntimeConfig, RuntimeFeatureConfig, RuntimeHookConfig, RuntimePluginConfig,
};
pub use conversation::{
    ApiClient, ApiRequest, AssistantEvent, ConversationRuntime, PermissionMode, PermissionPolicy,
    RuntimeError, StaticToolExecutor, ToolError, ToolExecutor, TurnSummary,
};
pub use hooks::{HookEvent, HookRunResult, HookRunner};
pub use oauth_support::{
    clear_oauth_credentials, load_oauth_credentials, save_oauth_credentials,
    OAuthRefreshRequest, OAuthTokenExchangeRequest, OAuthTokenSet,
};
pub use prompt::{
    load_system_prompt, prepend_bullets, ContextFile, ProjectContext, PromptBuildError,
    SystemPromptBuilder, FRONTIER_MODEL_NAME, SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
};
pub use session::{ContentBlock, ConversationMessage, MessageRole, Session, SessionError};
pub use smarttrade_tools::{
    classify_intent, detect_ambiguity, extract_strategy_spec, AmbiguityResult, AmbiguityStatus,
    run_static_analysis, save_strategy, search_knowledge_base, compile_mql5, CompileResult,
    CompilerMessage, GeneratedStrategy, GenerationError, IntentClassification, KnowledgeBaseMatch,
    KnowledgeBaseSearchResult, SaveStrategyRequest, SaveStrategyResult, SmartTradeToolConfig,
    SmartTradeToolExecutor, StaticAnalysisIssue, StaticAnalysisResult, StrategyIntent,
    StrategySpec, MAX_CLARIFICATION_ROUNDS, MAX_COMPILE_RETRIES, MAX_STATIC_RETRIES,
};
pub use usage::{
    format_usd, pricing_for_model, ModelPricing, TokenUsage, UsageCostEstimate, UsageTracker,
};

#[cfg(test)]
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
