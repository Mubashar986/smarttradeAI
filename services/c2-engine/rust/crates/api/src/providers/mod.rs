use std::future::Future;
use std::pin::Pin;

use crate::error::ApiError;
use crate::types::{MessageRequest, MessageResponse};

pub mod claw_provider;
pub mod openai_compat;

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

pub trait Provider {
    type Stream;

    fn send_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, MessageResponse>;

    fn stream_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, Self::Stream>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    ClawApi,
    Xai,
    OpenAi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub provider: ProviderKind,
    pub auth_env: &'static str,
    pub base_url_env: &'static str,
    pub default_base_url: &'static str,
}

const MODEL_REGISTRY: &[(&str, ProviderMetadata)] = &[
    (
        "opus",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "sonnet",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "haiku",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "claude-opus-4-6",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "claude-sonnet-4-6",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "claude-haiku-4-5-20251213",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "grok",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "grok-3",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "grok-mini",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "grok-3-mini",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "grok-2",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
];

#[must_use]
pub fn resolve_model_alias(model: &str) -> String {
    let trimmed = model.trim();
    let lower = trimmed.to_ascii_lowercase();
    MODEL_REGISTRY
        .iter()
        .find_map(|(alias, metadata)| {
            (*alias == lower).then_some(match metadata.provider {
                ProviderKind::ClawApi => match *alias {
                    "opus" => "claude-opus-4-6",
                    "sonnet" => "claude-sonnet-4-6",
                    "haiku" => "claude-haiku-4-5-20251213",
                    _ => trimmed,
                },
                ProviderKind::Xai => match *alias {
                    "grok" | "grok-3" => "grok-3",
                    "grok-mini" | "grok-3-mini" => "grok-3-mini",
                    "grok-2" => "grok-2",
                    _ => trimmed,
                },
                ProviderKind::OpenAi => trimmed,
            })
        })
        .map_or_else(|| trimmed.to_string(), ToOwned::to_owned)
}

#[must_use]
pub fn metadata_for_model(model: &str) -> Option<ProviderMetadata> {
    let canonical = resolve_model_alias(model);
    let lower = canonical.to_ascii_lowercase();
    if let Some((_, metadata)) = MODEL_REGISTRY.iter().find(|(alias, _)| *alias == lower) {
        return Some(*metadata);
    }
    if lower.starts_with("grok") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        });
    }
    None
}

fn parse_provider_override(value: &str) -> Option<ProviderKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "anthropic" | "claude" => Some(ProviderKind::ClawApi),
        "openai" | "groq" | "gemini" => Some(ProviderKind::OpenAi),
        "xai" => Some(ProviderKind::Xai),
        _ => None,
    }
}

fn provider_override_from_env() -> Option<ProviderKind> {
    std::env::var("LLM_PROVIDER")
        .ok()
        .and_then(|value| parse_provider_override(&value))
}

fn infer_provider_from_model_name(model: &str) -> Option<ProviderKind> {
    let lower = model.trim().to_ascii_lowercase();
    if lower.starts_with("claude")
        || lower.starts_with("opus")
        || lower.starts_with("sonnet")
        || lower.starts_with("haiku")
    {
        return Some(ProviderKind::ClawApi);
    }
    if lower.starts_with("grok") {
        return Some(ProviderKind::Xai);
    }
    if lower.starts_with("gpt")
        || lower.starts_with("gemini")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
        || lower.starts_with("llama")
        || lower.starts_with("qwen")
        || lower.starts_with("mixtral")
        || lower.starts_with("mistral")
        || lower.starts_with("gemma")
        || lower.starts_with("deepseek")
    {
        return Some(ProviderKind::OpenAi);
    }
    None
}

#[must_use]
pub fn detect_provider_kind(model: &str) -> ProviderKind {
    if let Some(metadata) = metadata_for_model(model) {
        return metadata.provider;
    }
    if let Some(override_kind) = provider_override_from_env() {
        return override_kind;
    }
    if let Some(inferred_kind) = infer_provider_from_model_name(model) {
        return inferred_kind;
    }
    if openai_compat::has_api_key("OPENAI_API_KEY") {
        return ProviderKind::OpenAi;
    }
    if openai_compat::has_api_key("XAI_API_KEY") {
        return ProviderKind::Xai;
    }
    if claw_provider::has_auth_from_env_or_saved().unwrap_or(false) {
        return ProviderKind::ClawApi;
    }
    ProviderKind::ClawApi
}

#[must_use]
pub fn selected_openai_compat_config(model: &str) -> openai_compat::OpenAiCompatConfig {
    if let Ok(value) = std::env::var("LLM_PROVIDER") {
        match value.trim().to_ascii_lowercase().as_str() {
            "groq" => return openai_compat::OpenAiCompatConfig::groq(),
            "gemini" => return openai_compat::OpenAiCompatConfig::gemini(),
            "openai" => return openai_compat::OpenAiCompatConfig::openai(),
            "xai" => return openai_compat::OpenAiCompatConfig::xai(),
            _ => {}
        }
    }

    let lower = resolve_model_alias(model).to_ascii_lowercase();
    if lower.starts_with("grok") {
        return openai_compat::OpenAiCompatConfig::xai();
    }
    if lower.starts_with("gemini") {
        return openai_compat::OpenAiCompatConfig::gemini();
    }

    if let Ok(base_url) =
        std::env::var("OPENAI_BASE_URL").or_else(|_| std::env::var("OPENAI_API_BASE"))
    {
        let lower_base = base_url.to_ascii_lowercase();
        if lower_base.contains("groq.com") {
            return openai_compat::OpenAiCompatConfig::groq();
        }
        if lower_base.contains("generativelanguage.googleapis.com") {
            return openai_compat::OpenAiCompatConfig::gemini();
        }
    }

    if openai_compat::has_api_key("GROQ_API_KEY") && !openai_compat::has_api_key("OPENAI_API_KEY")
    {
        return openai_compat::OpenAiCompatConfig::groq();
    }
    if openai_compat::has_api_key("GEMINI_API_KEY")
        && !openai_compat::has_api_key("OPENAI_API_KEY")
    {
        return openai_compat::OpenAiCompatConfig::gemini();
    }

    openai_compat::OpenAiCompatConfig::openai()
}

#[must_use]
pub fn max_tokens_for_model(model: &str) -> u32 {
    if let Ok(val) = std::env::var("CLAW_MAX_TOKENS") {
        if let Ok(tokens) = val.parse::<u32>() {
            return tokens;
        }
    }

    let canonical = resolve_model_alias(model);
    if canonical.contains("opus") {
        32_000
    } else {
        32_768 // Safe default for Groq/Llama/OpenAI
    }
}

#[cfg(test)]
mod tests {
    use super::{
        detect_provider_kind, infer_provider_from_model_name, max_tokens_for_model,
        parse_provider_override, resolve_model_alias, ProviderKind,
    };

    #[test]
    fn resolves_grok_aliases() {
        assert_eq!(resolve_model_alias("grok"), "grok-3");
        assert_eq!(resolve_model_alias("grok-mini"), "grok-3-mini");
        assert_eq!(resolve_model_alias("grok-2"), "grok-2");
    }

    #[test]
    fn detects_provider_from_model_name_first() {
        assert_eq!(detect_provider_kind("grok"), ProviderKind::Xai);
        assert_eq!(
            detect_provider_kind("claude-sonnet-4-6"),
            ProviderKind::ClawApi
        );
    }

    #[test]
    fn keeps_existing_max_token_heuristic() {
        assert_eq!(max_tokens_for_model("opus"), 32_000);
        assert_eq!(max_tokens_for_model("grok-3"), 32_768);
    }

    #[test]
    fn parses_provider_override_aliases() {
        assert_eq!(parse_provider_override("groq"), Some(ProviderKind::OpenAi));
        assert_eq!(parse_provider_override("openai"), Some(ProviderKind::OpenAi));
        assert_eq!(
            parse_provider_override("anthropic"),
            Some(ProviderKind::ClawApi)
        );
        assert_eq!(parse_provider_override("xai"), Some(ProviderKind::Xai));
        assert_eq!(parse_provider_override("unknown"), None);
    }

    #[test]
    fn infers_provider_from_model_prefix() {
        assert_eq!(
            infer_provider_from_model_name("llama-3.3-70b-versatile"),
            Some(ProviderKind::OpenAi)
        );
        assert_eq!(
            infer_provider_from_model_name("gpt-4.1"),
            Some(ProviderKind::OpenAi)
        );
        assert_eq!(
            infer_provider_from_model_name("claude-sonnet-4-6"),
            Some(ProviderKind::ClawApi)
        );
        assert_eq!(
            infer_provider_from_model_name("grok-3-mini"),
            Some(ProviderKind::Xai)
        );
    }

    #[test]
    fn model_prefix_inference_beats_env_credential_bias() {
        assert_eq!(
            detect_provider_kind("llama-3.3-70b-versatile"),
            ProviderKind::OpenAi
        );
    }
}
