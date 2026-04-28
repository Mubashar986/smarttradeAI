use std::ffi::OsStr;
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value as JsonValue};

use crate::config::{RuntimeFeatureConfig, RuntimeHookConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
}

impl HookEvent {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRunResult {
    denied: bool,
    messages: Vec<String>,
}

impl HookRunResult {
    #[must_use]
    pub fn allow(messages: Vec<String>) -> Self {
        Self {
            denied: false,
            messages,
        }
    }

    #[must_use]
    pub fn is_denied(&self) -> bool {
        self.denied
    }

    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

#[derive(Debug, Clone, Default)]
pub struct HookRunner {
    config: RuntimeHookConfig,
    smarttrade_gate_state: Arc<Mutex<std::collections::BTreeMap<String, SmartTradeGateState>>>,
}

#[derive(Debug, Clone, Copy)]
struct HookCommandRequest<'a> {
    event: HookEvent,
    tool_name: &'a str,
    tool_input: &'a str,
    tool_output: Option<&'a str>,
    is_error: bool,
    payload: &'a str,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SmartTradeGateState {
    static_passed: bool,
    compile_passed: bool,
}

impl HookRunner {
    #[must_use]
    pub fn new(config: RuntimeHookConfig) -> Self {
        Self {
            config,
            smarttrade_gate_state: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
        }
    }

    #[must_use]
    pub fn from_feature_config(feature_config: &RuntimeFeatureConfig) -> Self {
        Self::new(feature_config.hooks().clone())
    }

    #[must_use]
    pub fn run_pre_tool_use(&self, tool_name: &str, tool_input: &str) -> HookRunResult {
        let builtin = self.run_builtin_pre_tool_use(tool_name, tool_input);
        if builtin.is_denied() {
            return builtin;
        }

        let command_result = self.run_commands(
            HookEvent::PreToolUse,
            self.config.pre_tool_use(),
            tool_name,
            tool_input,
            None,
            false,
        );
        merge_hook_results(builtin, command_result)
    }

    #[must_use]
    pub fn run_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &str,
        tool_output: &str,
        is_error: bool,
    ) -> HookRunResult {
        let builtin = self.run_builtin_post_tool_use(tool_name, tool_input, tool_output, is_error);
        let command_result = self.run_commands(
            HookEvent::PostToolUse,
            self.config.post_tool_use(),
            tool_name,
            tool_input,
            Some(tool_output),
            is_error,
        );
        merge_hook_results(builtin, command_result)
    }

    fn run_builtin_pre_tool_use(&self, tool_name: &str, tool_input: &str) -> HookRunResult {
        if tool_name != "save_strategy" {
            return HookRunResult::allow(Vec::new());
        }

        let session_id = parse_session_id(tool_input).unwrap_or_else(|| "unknown".to_string());
        let state = self
            .smarttrade_gate_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&session_id)
            .copied()
            .unwrap_or_default();

        if state.static_passed && state.compile_passed {
            HookRunResult::allow(Vec::new())
        } else {
            HookRunResult {
                denied: true,
                messages: vec![format!(
                    "GATE DENIED: Cannot save strategy. Static analysis passed={}, Compilation passed={}. You MUST run run_static_analysis AND compile_mql5 first, and both must succeed, before calling save_strategy.",
                    state.static_passed,
                    state.compile_passed
                )],
            }
        }
    }

    fn run_builtin_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &str,
        tool_output: &str,
        is_error: bool,
    ) -> HookRunResult {
        let Some(session_id) = parse_session_id(tool_input) else {
            return HookRunResult::allow(Vec::new());
        };

        let mut gate_state = self
            .smarttrade_gate_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let state = gate_state.entry(session_id).or_default();

        match tool_name {
            "run_static_analysis" => {
                let passed = parse_bool_field(tool_output, "passed").unwrap_or(false);
                state.static_passed = !is_error && passed;
                state.compile_passed = false;
            }
            "compile_mql5" => {
                let success = parse_bool_field(tool_output, "success").unwrap_or(false);
                state.compile_passed = !is_error && success;
            }
            _ => {}
        }

        HookRunResult::allow(Vec::new())
    }

    fn run_commands(
        &self,
        event: HookEvent,
        commands: &[String],
        tool_name: &str,
        tool_input: &str,
        tool_output: Option<&str>,
        is_error: bool,
    ) -> HookRunResult {
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }

        let payload = json!({
            "hook_event_name": event.as_str(),
            "tool_name": tool_name,
            "tool_input": parse_tool_input(tool_input),
            "tool_input_json": tool_input,
            "tool_output": tool_output,
            "tool_result_is_error": is_error,
        })
        .to_string();

        let mut messages = Vec::new();

        for command in commands {
            match Self::run_command(
                command,
                HookCommandRequest {
                    event,
                    tool_name,
                    tool_input,
                    tool_output,
                    is_error,
                    payload: &payload,
                },
            ) {
                HookCommandOutcome::Allow { message } => {
                    if let Some(message) = message {
                        messages.push(message);
                    }
                }
                HookCommandOutcome::Deny { message } => {
                    let message = message.unwrap_or_else(|| {
                        format!("{} hook denied tool `{tool_name}`", event.as_str())
                    });
                    messages.push(message);
                    return HookRunResult {
                        denied: true,
                        messages,
                    };
                }
                HookCommandOutcome::Warn { message } => messages.push(message),
            }
        }

        HookRunResult::allow(messages)
    }

    fn run_command(command: &str, request: HookCommandRequest<'_>) -> HookCommandOutcome {
        let mut child = shell_command(command);
        child.stdin(std::process::Stdio::piped());
        child.stdout(std::process::Stdio::piped());
        child.stderr(std::process::Stdio::piped());
        child.env("HOOK_EVENT", request.event.as_str());
        child.env("HOOK_TOOL_NAME", request.tool_name);
        child.env("HOOK_TOOL_INPUT", request.tool_input);
        child.env(
            "HOOK_TOOL_IS_ERROR",
            if request.is_error { "1" } else { "0" },
        );
        if let Some(tool_output) = request.tool_output {
            child.env("HOOK_TOOL_OUTPUT", tool_output);
        }

        match child.output_with_stdin(request.payload.as_bytes()) {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let message = (!stdout.is_empty()).then_some(stdout);
                match output.status.code() {
                    Some(0) => HookCommandOutcome::Allow { message },
                    Some(2) => HookCommandOutcome::Deny { message },
                    Some(code) => HookCommandOutcome::Warn {
                        message: format_hook_warning(
                            command,
                            code,
                            message.as_deref(),
                            stderr.as_str(),
                        ),
                    },
                    None => HookCommandOutcome::Warn {
                        message: format!(
                            "{} hook `{command}` terminated by signal while handling `{}`",
                            request.event.as_str(),
                            request.tool_name
                        ),
                    },
                }
            }
            Err(error) => HookCommandOutcome::Warn {
                message: format!(
                    "{} hook `{command}` failed to start for `{}`: {error}",
                    request.event.as_str(),
                    request.tool_name
                ),
            },
        }
    }
}

enum HookCommandOutcome {
    Allow { message: Option<String> },
    Deny { message: Option<String> },
    Warn { message: String },
}

fn parse_tool_input(tool_input: &str) -> serde_json::Value {
    serde_json::from_str(tool_input).unwrap_or_else(|_| json!({ "raw": tool_input }))
}

fn parse_session_id(tool_input: &str) -> Option<String> {
    parse_tool_input(tool_input)
        .get("session_id")
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
}

fn parse_bool_field(tool_output: &str, field: &str) -> Option<bool> {
    serde_json::from_str::<JsonValue>(tool_output)
        .ok()
        .and_then(|value| value.get(field).and_then(JsonValue::as_bool))
}

fn merge_hook_results(left: HookRunResult, right: HookRunResult) -> HookRunResult {
    let denied = left.denied || right.denied;
    let mut messages = left.messages;
    messages.extend(right.messages);
    HookRunResult { denied, messages }
}

fn format_hook_warning(command: &str, code: i32, stdout: Option<&str>, stderr: &str) -> String {
    let mut message =
        format!("Hook `{command}` exited with status {code}; allowing tool execution to continue");
    if let Some(stdout) = stdout.filter(|stdout| !stdout.is_empty()) {
        message.push_str(": ");
        message.push_str(stdout);
    } else if !stderr.is_empty() {
        message.push_str(": ");
        message.push_str(stderr);
    }
    message
}

fn shell_command(command: &str) -> CommandWithStdin {
    #[cfg(windows)]
    let mut command_builder = {
        let mut command_builder = Command::new("cmd");
        command_builder.arg("/C").arg(command);
        CommandWithStdin::new(command_builder)
    };

    #[cfg(not(windows))]
    let command_builder = {
        let mut command_builder = Command::new("sh");
        command_builder.arg("-lc").arg(command);
        CommandWithStdin::new(command_builder)
    };

    command_builder
}

struct CommandWithStdin {
    command: Command,
}

impl CommandWithStdin {
    fn new(command: Command) -> Self {
        Self { command }
    }

    fn stdin(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stdin(cfg);
        self
    }

    fn stdout(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stdout(cfg);
        self
    }

    fn stderr(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stderr(cfg);
        self
    }

    fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.command.env(key, value);
        self
    }

    fn output_with_stdin(&mut self, stdin: &[u8]) -> std::io::Result<std::process::Output> {
        let mut child = self.command.spawn()?;
        if let Some(mut child_stdin) = child.stdin.take() {
            use std::io::Write;
            child_stdin.write_all(stdin)?;
        }
        child.wait_with_output()
    }
}

#[cfg(test)]
mod tests {
    use super::{HookRunResult, HookRunner};
    use crate::config::{RuntimeFeatureConfig, RuntimeHookConfig};

    #[test]
    fn allows_exit_code_zero_and_captures_stdout() {
        let runner = HookRunner::new(RuntimeHookConfig::new(
            vec![shell_snippet("printf 'pre ok'")],
            Vec::new(),
        ));

        let result = runner.run_pre_tool_use("Read", r#"{"path":"README.md"}"#);

        assert_eq!(result, HookRunResult::allow(vec!["pre ok".to_string()]));
    }

    #[test]
    fn denies_exit_code_two() {
        let runner = HookRunner::new(RuntimeHookConfig::new(
            vec![shell_snippet("printf 'blocked by hook'; exit 2")],
            Vec::new(),
        ));

        let result = runner.run_pre_tool_use("Bash", r#"{"command":"pwd"}"#);

        assert!(result.is_denied());
        assert_eq!(result.messages(), &["blocked by hook".to_string()]);
    }

    #[test]
    fn warns_for_other_non_zero_statuses() {
        let runner = HookRunner::from_feature_config(&RuntimeFeatureConfig::default().with_hooks(
            RuntimeHookConfig::new(
                vec![shell_snippet("printf 'warning hook'; exit 1")],
                Vec::new(),
            ),
        ));

        let result = runner.run_pre_tool_use("Edit", r#"{"file":"src/lib.rs"}"#);

        assert!(!result.is_denied());
        assert!(result
            .messages()
            .iter()
            .any(|message| message.contains("allowing tool execution to continue")));
    }

    #[test]
    fn denies_save_strategy_until_static_and_compile_have_passed() {
        let runner = HookRunner::default();

        let result = runner.run_pre_tool_use("save_strategy", r#"{"session_id":"sess-1"}"#);

        assert!(result.is_denied());
        assert!(result.messages()[0].contains("GATE DENIED"));
    }

    #[test]
    fn allows_save_strategy_after_static_and_compile_success() {
        let runner = HookRunner::default();

        let static_result = runner.run_post_tool_use(
            "run_static_analysis",
            r#"{"session_id":"sess-1"}"#,
            r#"{"passed":true}"#,
            false,
        );
        assert!(!static_result.is_denied());

        let compile_result = runner.run_post_tool_use(
            "compile_mql5",
            r#"{"session_id":"sess-1"}"#,
            r#"{"success":true}"#,
            false,
        );
        assert!(!compile_result.is_denied());

        let save_result = runner.run_pre_tool_use("save_strategy", r#"{"session_id":"sess-1"}"#);
        assert!(!save_result.is_denied());
    }

    #[test]
    fn rerunning_static_analysis_clears_previous_compile_gate() {
        let runner = HookRunner::default();

        runner.run_post_tool_use(
            "run_static_analysis",
            r#"{"session_id":"sess-1"}"#,
            r#"{"passed":true}"#,
            false,
        );
        runner.run_post_tool_use(
            "compile_mql5",
            r#"{"session_id":"sess-1"}"#,
            r#"{"success":true}"#,
            false,
        );
        assert!(!runner
            .run_pre_tool_use("save_strategy", r#"{"session_id":"sess-1"}"#)
            .is_denied());

        runner.run_post_tool_use(
            "run_static_analysis",
            r#"{"session_id":"sess-1"}"#,
            r#"{"passed":true}"#,
            false,
        );

        let result = runner.run_pre_tool_use("save_strategy", r#"{"session_id":"sess-1"}"#);
        assert!(result.is_denied());
        assert!(result.messages()[0].contains("Compilation passed=false"));
    }

    #[cfg(windows)]
    fn shell_snippet(script: &str) -> String {
        script.replace('\'', "\"")
    }

    #[cfg(not(windows))]
    fn shell_snippet(script: &str) -> String {
        script.to_string()
    }
}
