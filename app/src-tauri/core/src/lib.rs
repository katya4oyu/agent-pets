//! Pure, framework-free core for Agent Pets / Navi.
//!
//! This crate has **no** dependency on Tauri or any GUI/system library, so its
//! logic compiles and unit-tests headlessly (no `webkit2gtk` required). It owns:
//!
//! - the normalized event schema ([`AgentState`], [`HookEvent`]),
//! - hook payload normalization for Codex / Claude Code / Copilot ([`normalize`]),
//! - the [`WorldModel`] that aggregates active agent sessions.
//!
//! The Tauri app crate (`agent_pets_lib`) re-exports these types and drives the
//! core; the web frontend mirrors [`WorldModel`]'s aggregation in `state.ts`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

// ── Public event schema ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Thinking,
    Running,
    Editing,
    WaitingApproval,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub source: String,
    pub state: AgentState,
    pub label: String,
    pub message: Option<String>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub project_name: Option<String>,
    pub timestamp: Option<String>,
    pub terminal_program: Option<String>,
    pub terminal_session_id: Option<String>,
}

// ── Normalization input types ───────────────────────────────────────────────

struct HookInput {
    event_name: String,
    tool_name: Option<String>,
    tool_input: Option<Value>,
    message: Option<String>,
    error_message: Option<String>,
    notification_type: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
    terminal_program: Option<String>,
    terminal_session_id: Option<String>,
}

#[derive(Deserialize)]
struct SnakeHookPayload {
    hook_event_name: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_input: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    error_context: Option<String>,
    #[serde(default)]
    notification_type: Option<String>,
    #[serde(default)]
    terminal_program: Option<String>,
    #[serde(default)]
    terminal_session_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CopilotCamelPayload {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    initial_prompt: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_args: Option<Value>,
    #[serde(default)]
    tool_result: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
    #[serde(default)]
    error_context: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    agent_name: Option<String>,
    #[serde(default)]
    trigger: Option<String>,
    #[serde(default)]
    custom_instructions: Option<String>,
    #[serde(default)]
    notification_type: Option<String>,
}

// ── Normalization functions ─────────────────────────────────────────────────

fn str_val(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|s| s.as_str()).map(String::from)
}

fn value_message(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(String::from)
        .or_else(|| str_val(value, "message"))
        .or_else(|| str_val(value, "name"))
}

fn is_shell_tool(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "bash" | "shell" | "powershell" | "command"
    )
}

fn is_edit_tool(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "write" | "edit" | "apply_patch" | "create"
    )
}

fn map_state(event_name: &str, tool_name: Option<&str>) -> Option<(AgentState, &'static str)> {
    match event_name {
        "UserPromptSubmit" | "userPromptSubmitted" => Some((AgentState::Thinking, "Thinking")),
        "PreToolUse" | "preToolUse" => {
            let (state, label) = match tool_name {
                Some(n) if is_shell_tool(n) => (AgentState::Running, "Running shell"),
                Some(n) if is_edit_tool(n) => (AgentState::Editing, "Editing"),
                _ => (AgentState::Running, "Using tool"),
            };
            Some((state, label))
        }
        "PermissionRequest" | "permissionRequest" | "Elicitation" | "elicitation" => {
            Some((AgentState::WaitingApproval, "Waiting approval"))
        }
        "Notification" | "notification" => Some((AgentState::WaitingApproval, "Needs attention")),
        "PostToolUse" | "postToolUse" => Some((AgentState::Running, "Tool completed")),
        "PostToolUseFailure" | "postToolUseFailure" | "ErrorOccurred" | "errorOccurred"
        | "PermissionDenied" | "permissionDenied" => Some((AgentState::Error, "Tool failed")),
        "StopFailure" | "stopFailure" => Some((AgentState::Error, "Stopped with error")),
        "Stop" | "AgentStop" | "agentStop" | "SessionEnd" | "sessionEnd" | "SubagentStop"
        | "subagentStop" => Some((AgentState::Done, "Done")),
        "SessionStart" | "sessionStart" => Some((AgentState::Done, "Ready")),
        "SubagentStart" | "subagentStart" => Some((AgentState::Running, "Subagent starting")),
        "PreCompact" | "preCompact" => Some((AgentState::Thinking, "Compacting...")),
        "PostCompact" | "postCompact" => Some((AgentState::Done, "Compacted")),
        _ => None,
    }
}

fn extract_tool_message(tool_input: Option<&Value>, tool_name: Option<&str>) -> Option<String> {
    if let Some(input) = tool_input {
        if let Some(cmd) = str_val(input, "command") {
            return Some(cmd);
        }
        if let Some(fp) = str_val(input, "file_path").or_else(|| str_val(input, "filePath")) {
            return Some(fp);
        }
    }
    tool_name.map(String::from)
}

fn normalize_hook_input(input: HookInput, source: &str) -> Option<HookEvent> {
    let (mut state, mut label) = map_state(&input.event_name, input.tool_name.as_deref())?;

    if matches!(input.event_name.as_str(), "Notification" | "notification") {
        (state, label) = match input.notification_type.as_deref() {
            Some(
                "agent_idle"
                | "agent_completed"
                | "shell_completed"
                | "shell_detached_completed",
            ) => (AgentState::Done, "Done"),
            Some("permission_prompt" | "elicitation_dialog") => {
                (AgentState::WaitingApproval, "Waiting approval")
            }
            _ => (AgentState::WaitingApproval, "Needs attention"),
        };
    }

    if matches!(input.event_name.as_str(), "PostToolUse" | "postToolUse")
        && input.error_message.is_some()
    {
        state = AgentState::Error;
        label = "Tool failed";
    }

    let message = match input.event_name.as_str() {
        "UserPromptSubmit" | "userPromptSubmitted" => input.message,
        "Notification" | "notification" => input.message,
        "PreToolUse" | "preToolUse" => {
            extract_tool_message(input.tool_input.as_ref(), input.tool_name.as_deref())
        }
        "PostToolUseFailure" | "postToolUseFailure" | "ErrorOccurred" | "errorOccurred" => {
            input.error_message
        }
        "PostToolUse" | "postToolUse" if state == AgentState::Error => input.error_message,
        _ => None,
    };

    let project_name = input.cwd.as_deref().and_then(find_project_name);

    Some(HookEvent {
        source: source.to_string(),
        state,
        label: label.to_string(),
        message,
        session_id: input.session_id,
        cwd: input.cwd,
        project_name,
        timestamp: input.timestamp,
        terminal_program: input.terminal_program,
        terminal_session_id: input.terminal_session_id,
    })
}

fn snake_to_input(payload: SnakeHookPayload) -> HookInput {
    HookInput {
        event_name: payload.hook_event_name,
        tool_name: payload.tool_name,
        tool_input: payload.tool_input,
        message: payload.prompt.or(payload.message),
        error_message: payload
            .error
            .as_ref()
            .and_then(value_message)
            .or(payload.reason)
            .or(payload.error_context),
        notification_type: payload.notification_type,
        session_id: payload.session_id,
        cwd: payload.cwd,
        timestamp: payload.timestamp,
        terminal_program: payload.terminal_program,
        terminal_session_id: payload.terminal_session_id,
    }
}

fn infer_copilot_camel_event(payload: &CopilotCamelPayload) -> Option<&'static str> {
    if payload.prompt.is_some() {
        return Some("userPromptSubmitted");
    }
    if payload.tool_name.is_some() && payload.tool_result.is_some() {
        return Some("postToolUse");
    }
    if payload.tool_name.is_some() && payload.error.is_some() {
        return Some("postToolUseFailure");
    }
    if payload.tool_name.is_some() {
        return Some("preToolUse");
    }
    if payload.error.is_some() && payload.error_context.is_some() {
        return Some("errorOccurred");
    }
    if payload.trigger.is_some() && payload.custom_instructions.is_some() {
        return Some("preCompact");
    }
    if payload.agent_name.is_some() && payload.stop_reason.is_some() {
        return Some("subagentStop");
    }
    if payload.agent_name.is_some() {
        return Some("subagentStart");
    }
    if payload.stop_reason.is_some() {
        return Some("agentStop");
    }
    if payload.reason.is_some() {
        return Some("sessionEnd");
    }
    if payload.source.is_some() || payload.initial_prompt.is_some() {
        return Some("sessionStart");
    }
    None
}

fn copilot_camel_to_input(payload: CopilotCamelPayload) -> Option<HookInput> {
    let event_name = infer_copilot_camel_event(&payload)?.to_string();
    Some(HookInput {
        event_name,
        tool_name: payload.tool_name,
        tool_input: payload.tool_args,
        message: payload.prompt.or(payload.initial_prompt),
        error_message: payload
            .error
            .as_ref()
            .and_then(value_message)
            .or(payload.error_context),
        notification_type: payload.notification_type,
        session_id: payload.session_id,
        cwd: payload.cwd,
        timestamp: payload.timestamp.map(|ts| ts.to_string()),
        terminal_program: None,
        terminal_session_id: None,
    })
}

fn parse_hook_input(payload: &Value, source: &str) -> Option<HookInput> {
    if source == "copilot" && payload.get("hook_event_name").is_none() {
        let p: CopilotCamelPayload = serde_json::from_value(payload.clone()).ok()?;
        return copilot_camel_to_input(p);
    }
    let p: SnakeHookPayload = serde_json::from_value(payload.clone()).ok()?;
    Some(snake_to_input(p))
}

/// Normalize a raw agent hook payload into a [`HookEvent`].
///
/// Returns `None` for unknown / unsupported events (the caller should skip them).
pub fn normalize(payload: &Value, source: &str) -> Option<HookEvent> {
    let input = parse_hook_input(payload, source)?;
    normalize_hook_input(input, source)
}

fn find_project_name(cwd: &str) -> Option<String> {
    let mut path = std::path::Path::new(cwd);
    loop {
        if path.join(".git").exists() {
            return path.file_name().and_then(|n| n.to_str()).map(String::from);
        }
        let parent = path.parent()?;
        if parent == path {
            return None;
        }
        path = parent;
    }
}

// ── World Model ─────────────────────────────────────────────────────────────

/// Relative importance of each state when several sessions are active at once.
/// Mirrors `STATE_PRIORITY` in the frontend `state.ts` (higher wins).
pub fn state_priority(state: &AgentState) -> u8 {
    match state {
        AgentState::Error => 6,
        AgentState::WaitingApproval => 5,
        AgentState::Thinking => 4,
        AgentState::Running => 3,
        AgentState::Editing => 2,
        AgentState::Done => 1,
    }
}

/// Stable key grouping events into one session. Mirrors `sessionKey` in `state.ts`:
/// `"<source>:<session_id>"`, falling back to `source` alone when there is no id.
pub fn session_key(source: &str, session_id: Option<&str>) -> String {
    match session_id {
        Some(sid) => format!("{source}:{sid}"),
        None => source.to_string(),
    }
}

/// Backend-authoritative view of all currently active agent sessions.
///
/// This is the home (for Phase 1 of the Navi roadmap) of the session/aggregate
/// state that previously lived only in the frontend's `sessions` map. It is a
/// faithful port of that behavior; the frontend mirrors it in `state.ts`.
#[derive(Debug, Default, Clone)]
pub struct WorldModel {
    sessions: BTreeMap<String, AgentState>,
}

impl WorldModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a normalized event, updating (or inserting) the session's current
    /// state. Returns the session key that was touched.
    pub fn apply(&mut self, event: &HookEvent) -> String {
        let key = session_key(&event.source, event.session_id.as_deref());
        self.sessions.insert(key.clone(), event.state.clone());
        key
    }

    /// Forget a session (e.g. its bubble was dismissed). Returns whether a
    /// session was actually removed.
    pub fn remove(&mut self, key: &str) -> bool {
        self.sessions.remove(key).is_some()
    }

    pub fn clear(&mut self) {
        self.sessions.clear();
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// The most important state across all active sessions, used to drive the
    /// single pet animation. Resolves to [`AgentState::Done`] when no sessions
    /// are active. Mirrors `highestPriorityState` in `state.ts`.
    pub fn highest_priority_state(&self) -> AgentState {
        let mut best = AgentState::Done;
        for state in self.sessions.values() {
            if state_priority(state) > state_priority(&best) {
                best = state.clone();
            }
        }
        best
    }

    /// Current state of a specific session, if present.
    pub fn session_state(&self, key: &str) -> Option<&AgentState> {
        self.sessions.get(key)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- map_state ---

    #[test]
    fn user_prompt_submit_is_thinking() {
        assert_eq!(
            map_state("UserPromptSubmit", None),
            Some((AgentState::Thinking, "Thinking"))
        );
    }

    #[test]
    fn user_prompt_submitted_camel_is_thinking() {
        assert_eq!(
            map_state("userPromptSubmitted", None),
            Some((AgentState::Thinking, "Thinking"))
        );
    }

    #[test]
    fn pre_tool_use_bash_is_running_shell() {
        assert_eq!(
            map_state("PreToolUse", Some("Bash")),
            Some((AgentState::Running, "Running shell"))
        );
    }

    #[test]
    fn pre_tool_use_bash_lowercase_is_running_shell() {
        assert_eq!(
            map_state("preToolUse", Some("bash")),
            Some((AgentState::Running, "Running shell"))
        );
    }

    #[test]
    fn pre_tool_use_write_is_editing() {
        assert_eq!(
            map_state("PreToolUse", Some("Write")),
            Some((AgentState::Editing, "Editing"))
        );
    }

    #[test]
    fn pre_tool_use_edit_is_editing() {
        assert_eq!(
            map_state("PreToolUse", Some("edit")),
            Some((AgentState::Editing, "Editing"))
        );
    }

    #[test]
    fn pre_tool_use_apply_patch_is_editing() {
        assert_eq!(
            map_state("PreToolUse", Some("apply_patch")),
            Some((AgentState::Editing, "Editing"))
        );
    }

    #[test]
    fn pre_tool_use_mcp_tool_is_using_tool() {
        assert_eq!(
            map_state("PreToolUse", Some("mcp__server__do_thing")),
            Some((AgentState::Running, "Using tool"))
        );
    }

    #[test]
    fn permission_request_is_waiting_approval() {
        assert_eq!(
            map_state("PermissionRequest", None),
            Some((AgentState::WaitingApproval, "Waiting approval"))
        );
    }

    #[test]
    fn notification_is_needs_attention() {
        assert_eq!(
            map_state("Notification", None),
            Some((AgentState::WaitingApproval, "Needs attention"))
        );
    }

    #[test]
    fn notification_agent_idle_is_done() {
        let payload = json!({
            "hook_event_name": "Notification",
            "notification_type": "agent_idle",
            "message": "Claude is waiting for your input"
        });
        let event = normalize(&payload, "claude-code").unwrap();
        assert!(matches!(event.state, AgentState::Done));
        assert_eq!(event.label, "Done");
    }

    #[test]
    fn notification_agent_completed_is_done() {
        let payload = json!({
            "hook_event_name": "Notification",
            "notification_type": "agent_completed"
        });
        let event = normalize(&payload, "claude-code").unwrap();
        assert!(matches!(event.state, AgentState::Done));
    }

    #[test]
    fn notification_shell_completed_is_done() {
        for nt in ["shell_completed", "shell_detached_completed"] {
            let payload = json!({ "hook_event_name": "Notification", "notification_type": nt });
            let event = normalize(&payload, "claude-code").unwrap();
            assert!(matches!(event.state, AgentState::Done), "failed for {nt}");
        }
    }

    #[test]
    fn notification_permission_prompt_is_waiting_approval() {
        let payload = json!({
            "hook_event_name": "Notification",
            "notification_type": "permission_prompt"
        });
        let event = normalize(&payload, "claude-code").unwrap();
        assert!(matches!(event.state, AgentState::WaitingApproval));
        assert_eq!(event.label, "Waiting approval");
    }

    #[test]
    fn notification_elicitation_dialog_is_waiting_approval() {
        let payload = json!({
            "hook_event_name": "Notification",
            "notification_type": "elicitation_dialog"
        });
        let event = normalize(&payload, "claude-code").unwrap();
        assert!(matches!(event.state, AgentState::WaitingApproval));
        assert_eq!(event.label, "Waiting approval");
    }

    #[test]
    fn subagent_start_is_running() {
        assert_eq!(
            map_state("SubagentStart", None),
            Some((AgentState::Running, "Subagent starting"))
        );
        assert_eq!(
            map_state("subagentStart", None),
            Some((AgentState::Running, "Subagent starting"))
        );
    }

    #[test]
    fn pre_compact_is_thinking() {
        assert_eq!(
            map_state("PreCompact", None),
            Some((AgentState::Thinking, "Compacting..."))
        );
        assert_eq!(
            map_state("preCompact", None),
            Some((AgentState::Thinking, "Compacting..."))
        );
    }

    #[test]
    fn post_compact_is_done() {
        assert_eq!(
            map_state("PostCompact", None),
            Some((AgentState::Done, "Compacted"))
        );
    }

    #[test]
    fn stop_failure_is_error() {
        assert_eq!(
            map_state("StopFailure", None),
            Some((AgentState::Error, "Stopped with error"))
        );
    }

    #[test]
    fn post_tool_use_with_error_is_error() {
        let payload = json!({
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash",
            "error": "command not found"
        });
        let event = normalize(&payload, "codex").unwrap();
        assert!(matches!(event.state, AgentState::Error));
        assert_eq!(event.label, "Tool failed");
        assert_eq!(event.message.as_deref(), Some("command not found"));
    }

    #[test]
    fn project_name_from_git_root() {
        let project = std::env::var_os("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .and_then(|p| p.parent().map(|p| p.to_path_buf()));
        if let Some(root) = project {
            let name = find_project_name(root.to_str().unwrap());
            assert!(name.is_some(), "expected project name from git root");
        }
    }

    #[test]
    fn project_name_set_in_normalized_event() {
        let payload = json!({
            "hook_event_name": "UserPromptSubmit",
            "cwd": env!("CARGO_MANIFEST_DIR"),
            "prompt": "hello"
        });
        let event = normalize(&payload, "claude-code").unwrap();
        assert!(
            event.project_name.is_some(),
            "project_name should be set when cwd contains .git ancestor"
        );
    }

    #[test]
    fn terminal_fields_passed_through() {
        let payload = json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": "test",
            "terminal_program": "ghostty",
            "terminal_session_id": "abc123"
        });
        let event = normalize(&payload, "claude-code").unwrap();
        assert_eq!(event.terminal_program.as_deref(), Some("ghostty"));
        assert_eq!(event.terminal_session_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn post_tool_use_is_running() {
        assert_eq!(
            map_state("PostToolUse", None),
            Some((AgentState::Running, "Tool completed"))
        );
    }

    #[test]
    fn post_tool_use_failure_is_error() {
        assert_eq!(
            map_state("PostToolUseFailure", None),
            Some((AgentState::Error, "Tool failed"))
        );
    }

    #[test]
    fn stop_is_done() {
        assert_eq!(map_state("Stop", None), Some((AgentState::Done, "Done")));
    }

    #[test]
    fn agent_stop_camel_is_done() {
        assert_eq!(
            map_state("agentStop", None),
            Some((AgentState::Done, "Done"))
        );
    }

    #[test]
    fn session_start_is_done_ready() {
        assert_eq!(
            map_state("SessionStart", None),
            Some((AgentState::Done, "Ready"))
        );
    }

    #[test]
    fn unknown_event_returns_none() {
        assert_eq!(map_state("SomeFutureEvent", None), None);
    }

    #[test]
    fn normalize_claude_code_user_prompt() {
        let payload = json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "sess-1",
            "cwd": "/home/user/project",
            "prompt": "fix the bug"
        });
        let event = normalize(&payload, "claude-code").unwrap();
        assert!(matches!(event.state, AgentState::Thinking));
        assert_eq!(event.label, "Thinking");
        assert_eq!(event.message.as_deref(), Some("fix the bug"));
        assert_eq!(event.session_id.as_deref(), Some("sess-1"));
        assert_eq!(event.source, "claude-code");
    }

    #[test]
    fn normalize_codex_pre_tool_use_bash() {
        let payload = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "sess-2",
            "tool_name": "Bash",
            "tool_input": { "command": "cargo test" }
        });
        let event = normalize(&payload, "codex").unwrap();
        assert!(matches!(event.state, AgentState::Running));
        assert_eq!(event.message.as_deref(), Some("cargo test"));
    }

    #[test]
    fn normalize_codex_pre_tool_use_write() {
        let payload = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "tool_input": { "file_path": "src/main.rs", "content": "…" }
        });
        let event = normalize(&payload, "codex").unwrap();
        assert!(matches!(event.state, AgentState::Editing));
        assert_eq!(event.message.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn normalize_copilot_camel_case_fields() {
        let payload = json!({
            "sessionId": "copilot-sess",
            "timestamp": 1779361200000i64,
            "cwd": "/work",
            "prompt": "add tests"
        });
        let event = normalize(&payload, "copilot").unwrap();
        assert!(matches!(event.state, AgentState::Thinking));
        assert_eq!(event.message.as_deref(), Some("add tests"));
        assert_eq!(event.session_id.as_deref(), Some("copilot-sess"));
        assert_eq!(event.timestamp.as_deref(), Some("1779361200000"));
    }

    #[test]
    fn normalize_copilot_camel_pre_tool_use() {
        let payload = json!({
            "sessionId": "copilot-sess",
            "timestamp": 1779361200000i64,
            "cwd": "/work",
            "toolName": "bash",
            "toolArgs": { "command": "pnpm test" }
        });
        let event = normalize(&payload, "copilot").unwrap();
        assert!(matches!(event.state, AgentState::Running));
        assert_eq!(event.label, "Running shell");
        assert_eq!(event.message.as_deref(), Some("pnpm test"));
    }

    #[test]
    fn normalize_copilot_camel_post_tool_use_failure() {
        let payload = json!({
            "sessionId": "copilot-sess",
            "timestamp": 1779361200000i64,
            "cwd": "/work",
            "toolName": "bash",
            "toolArgs": { "command": "pnpm test" },
            "error": "command failed"
        });
        let event = normalize(&payload, "copilot").unwrap();
        assert!(matches!(event.state, AgentState::Error));
        assert_eq!(event.message.as_deref(), Some("command failed"));
    }

    #[test]
    fn normalize_copilot_pascal_pre_tool_use() {
        let payload = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "copilot-sess",
            "timestamp": "2026-05-21T00:00:00Z",
            "cwd": "/work",
            "tool_name": "edit",
            "tool_input": { "file_path": "src/main.ts" }
        });
        let event = normalize(&payload, "copilot").unwrap();
        assert!(matches!(event.state, AgentState::Editing));
        assert_eq!(event.message.as_deref(), Some("src/main.ts"));
    }

    #[test]
    fn normalize_missing_hook_event_name_returns_none() {
        let payload = json!({ "session_id": "abc", "prompt": "hi" });
        assert!(normalize(&payload, "codex").is_none());
    }

    #[test]
    fn normalize_unknown_event_returns_none() {
        let payload = json!({ "hook_event_name": "SomeFutureEvent" });
        assert!(normalize(&payload, "claude-code").is_none());
    }

    // --- WorldModel ---

    fn event(source: &str, session: Option<&str>, state: AgentState) -> HookEvent {
        HookEvent {
            source: source.to_string(),
            state,
            label: "test".to_string(),
            message: None,
            session_id: session.map(String::from),
            cwd: None,
            project_name: None,
            timestamp: None,
            terminal_program: None,
            terminal_session_id: None,
        }
    }

    #[test]
    fn session_key_combines_source_and_id() {
        assert_eq!(session_key("claude-code", Some("abc")), "claude-code:abc");
        assert_eq!(session_key("codex", None), "codex");
    }

    #[test]
    fn state_priority_follows_documented_order() {
        // error > waiting_approval > thinking > running > editing > done
        let ordered = [
            AgentState::Error,
            AgentState::WaitingApproval,
            AgentState::Thinking,
            AgentState::Running,
            AgentState::Editing,
            AgentState::Done,
        ];
        for pair in ordered.windows(2) {
            assert!(
                state_priority(&pair[0]) > state_priority(&pair[1]),
                "{:?} should outrank {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn world_model_empty_resolves_to_done() {
        let world = WorldModel::new();
        assert!(world.is_empty());
        assert_eq!(world.session_count(), 0);
        assert_eq!(world.highest_priority_state(), AgentState::Done);
    }

    #[test]
    fn world_model_tracks_single_session() {
        let mut world = WorldModel::new();
        let key = world.apply(&event("codex", Some("s1"), AgentState::Running));
        assert_eq!(key, "codex:s1");
        assert_eq!(world.session_count(), 1);
        assert_eq!(world.highest_priority_state(), AgentState::Running);
        assert_eq!(world.session_state("codex:s1"), Some(&AgentState::Running));
    }

    #[test]
    fn world_model_updates_state_in_place_for_same_session() {
        let mut world = WorldModel::new();
        world.apply(&event("codex", Some("s1"), AgentState::Running));
        world.apply(&event("codex", Some("s1"), AgentState::Done));
        assert_eq!(world.session_count(), 1);
        assert_eq!(world.highest_priority_state(), AgentState::Done);
    }

    #[test]
    fn world_model_highest_priority_across_sessions() {
        let mut world = WorldModel::new();
        world.apply(&event("codex", Some("s1"), AgentState::Done));
        world.apply(&event("claude-code", Some("s2"), AgentState::Running));
        world.apply(&event("copilot", Some("s3"), AgentState::WaitingApproval));
        assert_eq!(world.session_count(), 3);
        assert_eq!(world.highest_priority_state(), AgentState::WaitingApproval);
    }

    #[test]
    fn world_model_error_outranks_waiting_approval() {
        let mut world = WorldModel::new();
        world.apply(&event("codex", Some("s1"), AgentState::WaitingApproval));
        world.apply(&event("codex", Some("s2"), AgentState::Error));
        assert_eq!(world.highest_priority_state(), AgentState::Error);
    }

    #[test]
    fn world_model_no_session_id_collapses_to_source() {
        let mut world = WorldModel::new();
        world.apply(&event("codex", None, AgentState::Running));
        world.apply(&event("codex", None, AgentState::Done));
        assert_eq!(world.session_count(), 1, "same source w/o id is one session");
        assert_eq!(world.session_state("codex"), Some(&AgentState::Done));
    }

    #[test]
    fn world_model_remove_drops_session() {
        let mut world = WorldModel::new();
        let key = world.apply(&event("codex", Some("s1"), AgentState::Error));
        assert!(world.remove(&key));
        assert!(!world.remove(&key), "removing twice is a no-op");
        assert!(world.is_empty());
        assert_eq!(world.highest_priority_state(), AgentState::Done);
    }
}
