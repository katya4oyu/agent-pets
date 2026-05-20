use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct NormalizedEvent {
    source: String,
    state: &'static str,
    label: &'static str,
    message: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
}

fn str_val(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|s| s.as_str()).map(String::from)
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

fn map_state(event_name: &str, tool_name: Option<&str>) -> Option<(&'static str, &'static str)> {
    match event_name {
        "UserPromptSubmit" | "userPromptSubmitted" => Some(("thinking", "Thinking")),
        "PreToolUse" | "preToolUse" => {
            let (state, label) = match tool_name {
                Some(n) if is_shell_tool(n) => ("running", "Running shell"),
                Some(n) if is_edit_tool(n) => ("editing", "Editing"),
                _ => ("running", "Using tool"),
            };
            Some((state, label))
        }
        "PermissionRequest" | "permissionRequest" | "Elicitation" | "elicitation" => {
            Some(("waiting_approval", "Waiting approval"))
        }
        "Notification" | "notification" => Some(("waiting_approval", "Needs attention")),
        "PostToolUse" | "postToolUse" => Some(("running", "Tool completed")),
        "PostToolUseFailure" | "postToolUseFailure"
        | "ErrorOccurred" | "errorOccurred"
        | "PermissionDenied" | "permissionDenied" => Some(("error", "Tool failed")),
        "Stop" | "AgentStop" | "agentStop"
        | "SessionEnd" | "sessionEnd"
        | "SubagentStop" | "subagentStop" => Some(("done", "Done")),
        "SessionStart" | "sessionStart" => Some(("done", "Ready")),
        _ => None,
    }
}

fn extract_message(payload: &Value, event_name: &str, tool_name: Option<&str>) -> Option<String> {
    match event_name {
        "UserPromptSubmit" | "userPromptSubmitted" => str_val(payload, "prompt"),
        "Notification" | "notification" => str_val(payload, "message"),
        "PreToolUse" | "preToolUse" => {
            let input = payload.get("tool_input").or_else(|| payload.get("toolArgs"));
            if let Some(input) = input {
                if let Some(cmd) = str_val(input, "command") {
                    return Some(cmd);
                }
                if let Some(fp) = str_val(input, "file_path") {
                    return Some(fp);
                }
            }
            tool_name.map(String::from)
        }
        "PostToolUseFailure" | "postToolUseFailure"
        | "ErrorOccurred" | "errorOccurred" => {
            str_val(payload, "error")
                .or_else(|| str_val(payload, "reason"))
                .or_else(|| str_val(payload, "error_context"))
        }
        _ => None,
    }
}

fn normalize(payload: &Value, source: &str) -> Option<NormalizedEvent> {
    let event_name = str_val(payload, "hook_event_name")?;
    let tool_name = str_val(payload, "tool_name")
        .or_else(|| str_val(payload, "toolName"));
    let (state, label) = map_state(&event_name, tool_name.as_deref())?;
    let message = extract_message(payload, &event_name, tool_name.as_deref());
    let session_id = str_val(payload, "session_id")
        .or_else(|| str_val(payload, "sessionId"));
    let cwd = str_val(payload, "cwd");
    let timestamp = str_val(payload, "timestamp");

    Some(NormalizedEvent {
        source: source.to_string(),
        state,
        label,
        message,
        session_id,
        cwd,
        timestamp,
    })
}

fn read_port() -> Option<u16> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::PathBuf::from(home)
        .join(".agent-pets")
        .join("port");
    let text = std::fs::read_to_string(path).ok()?;
    text.trim().parse().ok()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "hook" {
        std::process::exit(1);
    }
    let source = args[2].clone();

    use std::io::Read;
    let mut body = String::new();
    if std::io::stdin().read_to_string(&mut body).is_err() {
        std::process::exit(0);
    }

    let payload: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => std::process::exit(0),
    };

    let event = match normalize(&payload, &source) {
        Some(e) => e,
        None => std::process::exit(0),
    };

    let port = match read_port() {
        Some(p) => p,
        None => std::process::exit(0),
    };

    let json = match serde_json::to_string(&event) {
        Ok(j) => j,
        Err(_) => std::process::exit(0),
    };

    let url = format!("http://127.0.0.1:{port}/events");
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_millis(200))
        .timeout(std::time::Duration::from_millis(200))
        .build();
    let _ = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(&json);

    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn user_prompt_submit_is_thinking() {
        assert_eq!(map_state("UserPromptSubmit", None), Some(("thinking", "Thinking")));
    }

    #[test]
    fn user_prompt_submitted_camel_is_thinking() {
        assert_eq!(map_state("userPromptSubmitted", None), Some(("thinking", "Thinking")));
    }

    #[test]
    fn pre_tool_use_bash_is_running_shell() {
        assert_eq!(
            map_state("PreToolUse", Some("Bash")),
            Some(("running", "Running shell"))
        );
    }

    #[test]
    fn pre_tool_use_bash_lowercase_is_running_shell() {
        assert_eq!(
            map_state("preToolUse", Some("bash")),
            Some(("running", "Running shell"))
        );
    }

    #[test]
    fn pre_tool_use_write_is_editing() {
        assert_eq!(
            map_state("PreToolUse", Some("Write")),
            Some(("editing", "Editing"))
        );
    }

    #[test]
    fn pre_tool_use_edit_is_editing() {
        assert_eq!(
            map_state("PreToolUse", Some("edit")),
            Some(("editing", "Editing"))
        );
    }

    #[test]
    fn pre_tool_use_apply_patch_is_editing() {
        assert_eq!(
            map_state("PreToolUse", Some("apply_patch")),
            Some(("editing", "Editing"))
        );
    }

    #[test]
    fn pre_tool_use_mcp_tool_is_using_tool() {
        assert_eq!(
            map_state("PreToolUse", Some("mcp__server__do_thing")),
            Some(("running", "Using tool"))
        );
    }

    #[test]
    fn permission_request_is_waiting_approval() {
        assert_eq!(
            map_state("PermissionRequest", None),
            Some(("waiting_approval", "Waiting approval"))
        );
    }

    #[test]
    fn notification_is_needs_attention() {
        assert_eq!(
            map_state("Notification", None),
            Some(("waiting_approval", "Needs attention"))
        );
    }

    #[test]
    fn post_tool_use_is_running() {
        assert_eq!(
            map_state("PostToolUse", None),
            Some(("running", "Tool completed"))
        );
    }

    #[test]
    fn post_tool_use_failure_is_error() {
        assert_eq!(
            map_state("PostToolUseFailure", None),
            Some(("error", "Tool failed"))
        );
    }

    #[test]
    fn stop_is_done() {
        assert_eq!(map_state("Stop", None), Some(("done", "Done")));
    }

    #[test]
    fn agent_stop_camel_is_done() {
        assert_eq!(map_state("agentStop", None), Some(("done", "Done")));
    }

    #[test]
    fn session_start_is_done_ready() {
        assert_eq!(map_state("SessionStart", None), Some(("done", "Ready")));
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
        assert_eq!(event.state, "thinking");
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
        assert_eq!(event.state, "running");
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
        assert_eq!(event.state, "editing");
        assert_eq!(event.message.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn normalize_copilot_camel_case_fields() {
        let payload = json!({
            "hook_event_name": "UserPromptSubmit",
            "sessionId": "copilot-sess",
            "cwd": "/work",
            "prompt": "add tests"
        });
        let event = normalize(&payload, "copilot").unwrap();
        assert_eq!(event.session_id.as_deref(), Some("copilot-sess"));
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
}
