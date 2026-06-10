use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, fs, path::PathBuf, process::Command, sync::Mutex};
use tauri::{Emitter, Manager};

// ── Public types ──────────────────────────────────────────────────────────────

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

#[derive(Default)]
struct TrayState {
    always_on_top: Mutex<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TrayMenuAction {
    ShowWindow,
    HideWindow,
    SetPet(String),
    SetPetSize(u16),
    SetSpeechMode(&'static str),
    InstallCli,
    SetupHooks(&'static str),
    ToggleAlwaysOnTop,
    OpenPetsFolder,
    Quit,
}

impl TrayMenuAction {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            id if id.starts_with("pet-select-") => Some(Self::SetPet(
                id.trim_start_matches("pet-select-").to_string(),
            )),
            "show-pet-window" => Some(Self::ShowWindow),
            "hide-pet-window" => Some(Self::HideWindow),
            "pet-size-small" => Some(Self::SetPetSize(96)),
            "pet-size-medium" => Some(Self::SetPetSize(128)),
            "pet-size-large" => Some(Self::SetPetSize(176)),
            "speech-show" => Some(Self::SetSpeechMode("show")),
            "speech-hide" => Some(Self::SetSpeechMode("hide")),
            "speech-auto" => Some(Self::SetSpeechMode("auto")),
            "install-cli-tool" => Some(Self::InstallCli),
            "setup-hooks-all" => Some(Self::SetupHooks("all")),
            "setup-hooks-claude-code" => Some(Self::SetupHooks("claude-code")),
            "setup-hooks-codex" => Some(Self::SetupHooks("codex")),
            "setup-hooks-copilot" => Some(Self::SetupHooks("copilot")),
            "always-on-top" => Some(Self::ToggleAlwaysOnTop),
            "open-pets-folder" => Some(Self::OpenPetsFolder),
            "quit-agent-pets" => Some(Self::Quit),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PetSizePayload {
    size: u16,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeechModePayload {
    mode: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PetSelectionPayload {
    pet_id: String,
}

pub fn is_valid_hook_source(source: &str) -> bool {
    matches!(source, "claude-code" | "codex" | "copilot")
}

pub fn cli_info() -> String {
    format!("agent-pets {}", env!("CARGO_PKG_VERSION"))
}

pub fn read_agent_pets_port() -> Option<u16> {
    let path = port_file_path()?;
    read_agent_pets_port_at(&path)
}

fn read_agent_pets_port_at(path: &std::path::Path) -> Option<u16> {
    fs::read_to_string(path).ok()?.trim().parse::<u16>().ok()
}

// ── Normalization types ───────────────────────────────────────────────────────

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

// ── Normalization functions ───────────────────────────────────────────────────

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

fn normalize(payload: &Value, source: &str) -> Option<HookEvent> {
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

// ── Port file management ──────────────────────────────────────────────────────

fn agent_pets_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".agent-pets"))
}

fn port_file_path() -> Option<PathBuf> {
    agent_pets_dir().map(|d| d.join("port"))
}

fn cleanup_stale_port_file() {
    if let Some(path) = port_file_path() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            let is_stale = text.trim().parse::<u16>().map_or(true, |port| {
                std::net::TcpStream::connect(std::net::SocketAddr::from(([127, 0, 0, 1], port)))
                    .is_err()
            });
            if is_stale {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

fn write_port_file(port: u16) {
    if let Some(path) = port_file_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, port.to_string());
    }
}

fn remove_port_file() {
    if let Some(path) = port_file_path() {
        let _ = std::fs::remove_file(path);
    }
}

// ── HTTP server ───────────────────────────────────────────────────────────────

fn parse_event_source_from_path(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let Some(source) = path.strip_prefix("/events/") else {
        return String::new();
    };
    match source {
        "claude-code" | "codex" | "copilot" => source.to_string(),
        _ => String::new(),
    }
}

fn start_event_server(app_handle: tauri::AppHandle) {
    use tauri::Emitter;

    std::thread::spawn(move || {
        cleanup_stale_port_file();

        let port = match std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr().map(|a| a.port()))
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("agent-pets: could not find free port: {e}");
                return;
            }
        };
        write_port_file(port);

        let server = match tiny_http::Server::http(format!("127.0.0.1:{port}")) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("agent-pets: HTTP server failed to start: {e}");
                remove_port_file();
                return;
            }
        };

        for mut request in server.incoming_requests() {
            let source = parse_event_source_from_path(request.url());
            if request.method() != &tiny_http::Method::Post || source.is_empty() {
                let _ = request
                    .respond(tiny_http::Response::from_string("not found").with_status_code(404));
                continue;
            }

            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_err() {
                let _ = request
                    .respond(tiny_http::Response::from_string("bad request").with_status_code(400));
                continue;
            }

            let payload: Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => {
                    let _ = request.respond(
                        tiny_http::Response::from_string("bad request").with_status_code(400),
                    );
                    continue;
                }
            };

            if let Some(event) = normalize(&payload, &source) {
                let _ = app_handle.emit("agent-state-changed", &event);
            }
            let _ = request.respond(tiny_http::Response::from_string("ok"));
        }
    });
}

// ── Setup helpers ─────────────────────────────────────────────────────────────

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME 環境変数が設定されていません".to_string())
}

fn read_json_or_empty(path: &std::path::Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("{} のパースに失敗: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Value::Object(Default::default())),
        Err(e) => Err(format!("{} の読み込みに失敗: {e}", path.display())),
    }
}

fn write_json_atomic(path: &std::path::Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("ディレクトリ作成に失敗: {e}"))?;
    }
    let json =
        serde_json::to_string_pretty(value).map_err(|e| format!("JSON シリアライズに失敗: {e}"))?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json)
        .map_err(|e| format!("{} への書き込みに失敗: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("rename に失敗: {e}"))?;
    Ok(())
}

fn upsert_codex_hook(hooks_obj: &mut serde_json::Map<String, Value>, event: &str, cmd: &str) {
    let arr = hooks_obj
        .entry(event.to_string())
        .or_insert_with(|| Value::Array(vec![]));
    if let Some(arr) = arr.as_array_mut() {
        arr.retain(|group| {
            !group
                .get("hooks")
                .and_then(Value::as_array)
                .map_or(false, |inner| {
                    inner.iter().any(|h| {
                        h.get("command")
                            .and_then(Value::as_str)
                            .map_or(false, |c| c.contains("agent-pets"))
                    })
                })
        });
        arr.push(serde_json::json!({
            "hooks": [{"type": "command", "command": cmd, "timeout": 1}]
        }));
    }
}

fn upsert_claude_code_hook(hooks_obj: &mut serde_json::Map<String, Value>, event: &str, cmd: &str) {
    let arr = hooks_obj
        .entry(event.to_string())
        .or_insert_with(|| Value::Array(vec![]));
    if let Some(arr) = arr.as_array_mut() {
        arr.retain(|group| {
            let in_nested = group
                .get("hooks")
                .and_then(Value::as_array)
                .map_or(false, |inner| {
                    inner.iter().any(|h| {
                        h.get("command")
                            .and_then(Value::as_str)
                            .map_or(false, |c| c.contains("agent-pets"))
                    })
                });
            let in_flat = group
                .get("command")
                .and_then(Value::as_str)
                .map_or(false, |c| c.contains("agent-pets"));
            !(in_nested || in_flat)
        });
        arr.push(serde_json::json!({
            "matcher": "",
            "hooks": [{"type": "command", "command": cmd, "async": true, "timeout": 1}]
        }));
    }
}

fn is_agent_pets_command(value: &Value) -> bool {
    ["command", "bash", "powershell"].iter().any(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .is_some_and(|cmd| cmd.contains("agent-pets"))
    })
}

fn remove_agent_pets_from_hook_array(arr: &mut Vec<Value>) -> usize {
    let before = arr.len();
    arr.retain(|entry| !is_agent_pets_command(entry));
    before - arr.len()
}

pub fn remove_agent_pets_hooks_from_codex(root: &mut Value) -> usize {
    let Some(root_obj) = root.as_object_mut() else {
        return 0;
    };

    let mut removed = 0;

    // New format: root["hooks"][EventName] = [matcher groups]
    if let Some(hooks_sub) = root_obj.get_mut("hooks").and_then(Value::as_object_mut) {
        for groups in hooks_sub.values_mut().filter_map(Value::as_array_mut) {
            let before = groups.len();
            groups.retain(|group| {
                !group
                    .get("hooks")
                    .and_then(Value::as_array)
                    .map_or(false, |inner| {
                        inner.iter().any(|h| {
                            h.get("command")
                                .and_then(Value::as_str)
                                .map_or(false, |c| c.contains("agent-pets"))
                        })
                    })
            });
            removed += before - groups.len();
        }
    }

    // Old flat format: root[EventName] = [command entries] (backward compat migration)
    for (key, value) in root_obj.iter_mut() {
        if key == "hooks" {
            continue;
        }
        if let Some(arr) = value.as_array_mut() {
            removed += remove_agent_pets_from_hook_array(arr);
        }
    }

    removed
}

pub fn remove_agent_pets_hooks_from_claude_settings(settings: &mut Value) -> usize {
    let Some(hooks_obj) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return 0;
    };

    let mut removed = 0;
    for groups in hooks_obj.values_mut().filter_map(Value::as_array_mut) {
        groups.retain_mut(|group| {
            if is_agent_pets_command(group) {
                removed += 1;
                return false;
            }
            if let Some(inner) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                removed += remove_agent_pets_from_hook_array(inner);
                return !inner.is_empty();
            }
            true
        });
    }
    removed
}

fn value_contains_agent_pets(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("agent-pets"),
        Value::Array(items) => items.iter().any(value_contains_agent_pets),
        Value::Object(obj) => obj.values().any(value_contains_agent_pets),
        _ => false,
    }
}

fn value_contains_non_agent_pets_hook_command(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(value_contains_non_agent_pets_hook_command),
        Value::Object(obj) => {
            let command_value = obj
                .get("command")
                .or_else(|| obj.get("bash"))
                .or_else(|| obj.get("powershell"))
                .and_then(Value::as_str);
            command_value.is_some_and(|cmd| !cmd.contains("agent-pets"))
                || obj.values().any(value_contains_non_agent_pets_hook_command)
        }
        _ => false,
    }
}

pub fn is_agent_pets_copilot_config(config: &Value) -> bool {
    value_contains_agent_pets(config) && !value_contains_non_agent_pets_hook_command(config)
}

fn cli_install_path() -> Result<PathBuf, String> {
    let home = home_dir()?;
    Ok(home.join(".agent-pets").join("bin").join("agent-pets"))
}

fn cli_source_path() -> Result<PathBuf, String> {
    let current =
        env::current_exe().map_err(|error| format!("現在の実行ファイル取得に失敗: {error}"))?;
    let file_name = if cfg!(windows) {
        "agent-pets-hook.exe"
    } else {
        "agent-pets-hook"
    };
    let sibling = current.with_file_name(file_name);
    if sibling.is_file() {
        return Ok(sibling);
    }
    Err(format!(
        "CLI バイナリが見つかりません: {}",
        sibling.display()
    ))
}

fn shell_quote(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn hook_command(source: &str, cli_path: &std::path::Path) -> String {
    format!("{} hook {source}", shell_quote(cli_path))
}

fn validate_cli_tool(path: &std::path::Path) -> Result<(), String> {
    let output = Command::new(path)
        .arg("cli-info")
        .output()
        .map_err(|error| format!("CLI の検証実行に失敗: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "CLI の検証に失敗しました: exit status {}",
            output.status
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim_start().starts_with("agent-pets ") {
        return Err("CLI の検証出力が不正です".to_string());
    }
    Ok(())
}

#[tauri::command]
fn install_cli_tool() -> Result<String, String> {
    let source = cli_source_path()?;
    let destination = cli_install_path()?;
    if source == destination {
        validate_cli_tool(&destination)?;
        return Ok(format!("Installed CLI: {}", destination.display()));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("CLI ディレクトリ作成に失敗: {error}"))?;
    }
    fs::copy(&source, &destination).map_err(|error| {
        format!(
            "{} から {} への CLI コピーに失敗: {error}",
            source.display(),
            destination.display()
        )
    })?;
    validate_cli_tool(&destination)?;
    Ok(format!("Installed CLI: {}", destination.display()))
}

fn ensure_cli_tool_installed() -> Result<PathBuf, String> {
    let path = cli_install_path()?;
    if validate_cli_tool(&path).is_err() {
        install_cli_tool()?;
    }
    Ok(path)
}

fn setup_claude_code(cmd: &str) -> Result<String, String> {
    let home = home_dir()?;
    let path = home.join(".claude").join("settings.json");

    let mut settings = read_json_or_empty(&path)?;
    {
        let obj = settings
            .as_object_mut()
            .ok_or("settings.json はオブジェクトではありません")?;
        let hooks = obj
            .entry("hooks")
            .or_insert_with(|| Value::Object(Default::default()))
            .as_object_mut()
            .ok_or("hooks フィールドはオブジェクトではありません")?;

        for event in [
            "UserPromptSubmit",
            "PreToolUse",
            "PermissionRequest",
            "PostToolUse",
            "PostToolUseFailure",
            "Notification",
            "Stop",
            "SubagentStart",
            "SubagentStop",
            "PreCompact",
            "PostCompact",
        ] {
            upsert_claude_code_hook(hooks, event, cmd);
        }
    }

    write_json_atomic(&path, &settings)?;
    Ok(format!("Claude Code: {}", path.display()))
}

fn setup_codex(cmd: &str) -> Result<String, String> {
    let home = home_dir()?;
    let path = home.join(".codex").join("hooks.json");

    let mut root = read_json_or_empty(&path)?;
    {
        let root_obj = root
            .as_object_mut()
            .ok_or("hooks.json はオブジェクトではありません")?;

        // Remove old flat-format entries at root level (migration)
        for arr in root_obj.values_mut().filter_map(Value::as_array_mut) {
            arr.retain(|e| !is_agent_pets_command(e));
        }

        let hooks_sub = root_obj
            .entry("hooks")
            .or_insert_with(|| Value::Object(Default::default()))
            .as_object_mut()
            .ok_or("hooks フィールドはオブジェクトではありません")?;

        for event in [
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PermissionRequest",
            "PostToolUse",
            "Stop",
            "SubagentStart",
            "SubagentStop",
            "PreCompact",
            "PostCompact",
        ] {
            upsert_codex_hook(hooks_sub, event, cmd);
        }
    }

    write_json_atomic(&path, &root)?;
    Ok(format!("Codex: {}", path.display()))
}

fn setup_copilot(cmd: &str) -> Result<String, String> {
    let home = home_dir()?;
    let path = home.join(".copilot").join("hooks").join("agent-pets.json");

    let mut hooks_obj = serde_json::Map::new();
    for event in [
        "UserPromptSubmit",
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PostToolUseFailure",
        "Notification",
        "Stop",
        "ErrorOccurred",
    ] {
        hooks_obj.insert(
            event.to_string(),
            serde_json::json!([{"bash": cmd, "timeoutSec": 1}]),
        );
    }
    let config = serde_json::json!({"version": 1, "hooks": hooks_obj});

    write_json_atomic(&path, &config)?;
    Ok(format!("Copilot: {}", path.display()))
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn setup_hooks(agent: String) -> Result<String, String> {
    let cli_path = ensure_cli_tool_installed()?;
    let make_cmd = |source: &str| hook_command(source, &cli_path);

    let mut messages: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    messages.push(format!("CLI: {}", cli_path.display()));
    let mut try_run = |result: Result<String, String>| match result {
        Ok(msg) => messages.push(msg),
        Err(e) => errors.push(e),
    };

    match agent.as_str() {
        "claude-code" => try_run(setup_claude_code(&make_cmd("claude-code"))),
        "codex" => try_run(setup_codex(&make_cmd("codex"))),
        "copilot" => try_run(setup_copilot(&make_cmd("copilot"))),
        "all" => {
            try_run(setup_claude_code(&make_cmd("claude-code")));
            try_run(setup_codex(&make_cmd("codex")));
            try_run(setup_copilot(&make_cmd("copilot")));
        }
        other => return Err(format!("不明なエージェント: {other}")),
    }

    if !errors.is_empty() {
        Err(errors.join("\n"))
    } else {
        Ok(messages.join("\n"))
    }
}

fn pets_dir() -> Option<PathBuf> {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .map(|home| home.join("pets"))
}

fn list_pet_ids() -> Vec<String> {
    let Some(pets_dir) = pets_dir() else {
        return vec!["mio".to_string()];
    };
    let mut ids = fs::read_dir(pets_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter_map(|entry| {
            let path = entry.path();
            if path.join("pet.json").is_file() {
                entry.file_name().to_str().map(str::to_string)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        ids.push("mio".to_string());
    }
    ids
}

fn emit_setup_result(app: &tauri::AppHandle, result: Result<String, String>) {
    let (label, message) = match result {
        Ok(message) => ("Hooks configured", message),
        Err(message) => ("Hook setup failed", message),
    };
    let event = HookEvent {
        source: "agent-pets".to_string(),
        state: if label == "Hooks configured" {
            AgentState::Done
        } else {
            AgentState::Error
        },
        label: label.to_string(),
        message: Some(message),
        session_id: None,
        cwd: None,
        project_name: None,
        timestamp: None,
        terminal_program: None,
        terminal_session_id: None,
    };
    let _ = app.emit("agent-state-changed", event);
}

fn handle_tray_action(app: &tauri::AppHandle, action: TrayMenuAction) {
    let window = || app.get_webview_window("main");
    match action {
        TrayMenuAction::ShowWindow => {
            if let Some(window) = window() {
                let _ = window.show();
            }
        }
        TrayMenuAction::HideWindow => {
            if let Some(window) = window() {
                let _ = window.hide();
            }
        }
        TrayMenuAction::SetPet(pet_id) => {
            let _ = app.emit("set-pet", PetSelectionPayload { pet_id });
        }
        TrayMenuAction::SetPetSize(size) => {
            let _ = app.emit("set-pet-size", PetSizePayload { size });
        }
        TrayMenuAction::SetSpeechMode(mode) => {
            let _ = app.emit("set-speech-mode", SpeechModePayload { mode });
        }
        TrayMenuAction::InstallCli => {
            emit_setup_result(app, install_cli_tool());
        }
        TrayMenuAction::SetupHooks(agent) => {
            emit_setup_result(app, setup_hooks(agent.to_string()));
        }
        TrayMenuAction::ToggleAlwaysOnTop => {
            let state = app.state::<TrayState>();
            let mut always_on_top = state.always_on_top.lock().unwrap();
            *always_on_top = !*always_on_top;
            if let Some(window) = window() {
                let _ = window.set_always_on_top(*always_on_top);
            }
        }
        TrayMenuAction::OpenPetsFolder => {
            if let Some(home) = env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
            {
                let pets_dir = home.join("pets");
                let _ = fs::create_dir_all(&pets_dir);
                let _ = tauri_plugin_opener::open_path(pets_dir, None::<&str>);
            }
        }
        TrayMenuAction::Quit => {
            remove_port_file();
            app.exit(0);
        }
    }
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    use tauri::menu::{CheckMenuItem, MenuBuilder, MenuItem, SubmenuBuilder};
    use tauri::tray::TrayIconBuilder;

    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let size_menu = SubmenuBuilder::new(app, "Pet Size")
        .text("pet-size-small", "Small")
        .text("pet-size-medium", "Medium")
        .text("pet-size-large", "Large")
        .build()?;
    let mut character_menu = SubmenuBuilder::new(app, "Character");
    for pet_id in list_pet_ids() {
        character_menu = character_menu.text(format!("pet-select-{pet_id}"), pet_id);
    }
    let character_menu = character_menu.build()?;
    let speech_menu = SubmenuBuilder::new(app, "Speech Bubble")
        .text("speech-show", "Show")
        .text("speech-hide", "Hide")
        .text("speech-auto", "Auto")
        .build()?;
    let setup_menu = SubmenuBuilder::new(app, "Setup Hooks")
        .text("setup-hooks-all", "All Agents")
        .text("setup-hooks-claude-code", "Claude Code")
        .text("setup-hooks-codex", "Codex")
        .text("setup-hooks-copilot", "Copilot")
        .build()?;
    let always_on_top = CheckMenuItem::with_id(
        app,
        "always-on-top",
        "Always On Top",
        true,
        true,
        None::<&str>,
    )?;
    let title = MenuItem::with_id(app, "agent-pets-title", "Agent Pets", false, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&title)
        .separator()
        .text("show-pet-window", "Show Pet Window")
        .text("hide-pet-window", "Hide Pet Window")
        .separator()
        .item(&character_menu)
        .item(&size_menu)
        .item(&speech_menu)
        .text("install-cli-tool", "Install CLI Tool")
        .item(&setup_menu)
        .separator()
        .item(&always_on_top)
        .text("open-pets-folder", "Open Pets Folder")
        .separator()
        .text("quit-agent-pets", "Quit Agent Pets")
        .build()?;

    let mut tray = TrayIconBuilder::with_id("agent-pets").menu(&menu);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(move |app, event| {
        if let Some(action) = TrayMenuAction::from_id(event.id().as_ref()) {
            if action == TrayMenuAction::ToggleAlwaysOnTop {
                let state = app.state::<TrayState>();
                let checked = {
                    let always_on_top = state.always_on_top.lock().unwrap();
                    !*always_on_top
                };
                let _ = always_on_top.set_checked(checked);
            }
            handle_tray_action(app, action);
        }
    })
    .build(app)?;

    Ok(())
}

#[tauri::command]
fn ping(message: &str) -> String {
    format!("{message} is listening.")
}

#[tauri::command]
fn load_pet_asset(pet_id: Option<String>) -> Result<PetAsset, String> {
    let pet_id = pet_id.unwrap_or_else(|| "mio".to_string());
    let home = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .ok_or_else(|| "Could not find CODEX_HOME or HOME".to_string())?;
    let pet_dir = home.join("pets").join(&pet_id);
    let manifest_path = pet_dir.join("pet.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Failed to read {}: {error}", manifest_path.display()))?;
    let manifest: PetManifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("Failed to parse {}: {error}", manifest_path.display()))?;
    let spritesheet_path = pet_dir.join(&manifest.spritesheet_path);
    let spritesheet_bytes = fs::read(&spritesheet_path)
        .map_err(|error| format!("Failed to read {}: {error}", spritesheet_path.display()))?;
    let spritesheet_mime = match spritesheet_path.extension().and_then(|ext| ext.to_str()) {
        Some("png") => "image/png",
        _ => "image/webp",
    }
    .to_string();

    Ok(PetAsset {
        id: manifest.id,
        display_name: manifest.display_name,
        description: manifest.description,
        spritesheet_path: spritesheet_path.display().to_string(),
        spritesheet_mime,
        spritesheet_bytes,
    })
}

// ── Pet asset types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PetAsset {
    id: String,
    display_name: String,
    description: String,
    spritesheet_path: String,
    spritesheet_mime: String,
    spritesheet_bytes: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PetManifest {
    id: String,
    display_name: String,
    description: String,
    spritesheet_path: String,
}

// ── App entry point ───────────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .manage(TrayState {
            always_on_top: Mutex::new(true),
        })
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_tray(app)?;
            let handle = app.handle().clone();
            start_event_server(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_pet_asset,
            ping,
            install_cli_tool,
            setup_hooks
        ])
        .build(tauri::generate_context!())
        .expect("error building Agent Pets")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                remove_port_file();
            }
        });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn parse_event_source_from_path_extracts_source() {
        assert_eq!(
            parse_event_source_from_path("/events/claude-code"),
            "claude-code"
        );
        assert_eq!(parse_event_source_from_path("/events/codex"), "codex");
        assert_eq!(parse_event_source_from_path("/events/copilot"), "copilot");
        assert_eq!(parse_event_source_from_path("/events"), "");
        assert_eq!(parse_event_source_from_path("/events?source=codex"), "");
    }

    #[test]
    fn tray_menu_action_maps_size_items() {
        assert_eq!(
            TrayMenuAction::from_id("pet-size-small"),
            Some(TrayMenuAction::SetPetSize(96))
        );
        assert_eq!(
            TrayMenuAction::from_id("pet-size-medium"),
            Some(TrayMenuAction::SetPetSize(128))
        );
        assert_eq!(
            TrayMenuAction::from_id("pet-size-large"),
            Some(TrayMenuAction::SetPetSize(176))
        );
    }

    #[test]
    fn tray_menu_action_maps_speech_modes() {
        assert_eq!(
            TrayMenuAction::from_id("speech-show"),
            Some(TrayMenuAction::SetSpeechMode("show"))
        );
        assert_eq!(
            TrayMenuAction::from_id("speech-hide"),
            Some(TrayMenuAction::SetSpeechMode("hide"))
        );
        assert_eq!(
            TrayMenuAction::from_id("speech-auto"),
            Some(TrayMenuAction::SetSpeechMode("auto"))
        );
    }

    #[test]
    fn tray_menu_action_maps_setup_hooks() {
        assert_eq!(
            TrayMenuAction::from_id("install-cli-tool"),
            Some(TrayMenuAction::InstallCli)
        );
        assert_eq!(
            TrayMenuAction::from_id("setup-hooks-all"),
            Some(TrayMenuAction::SetupHooks("all"))
        );
        assert_eq!(
            TrayMenuAction::from_id("setup-hooks-claude-code"),
            Some(TrayMenuAction::SetupHooks("claude-code"))
        );
        assert_eq!(
            TrayMenuAction::from_id("setup-hooks-codex"),
            Some(TrayMenuAction::SetupHooks("codex"))
        );
        assert_eq!(
            TrayMenuAction::from_id("setup-hooks-copilot"),
            Some(TrayMenuAction::SetupHooks("copilot"))
        );
    }

    #[test]
    fn tray_menu_action_maps_pet_selection() {
        assert_eq!(
            TrayMenuAction::from_id("pet-select-mio"),
            Some(TrayMenuAction::SetPet("mio".to_string()))
        );
        assert_eq!(
            TrayMenuAction::from_id("pet-select-bitomos-front"),
            Some(TrayMenuAction::SetPet("bitomos-front".to_string()))
        );
    }

    #[test]
    fn hook_source_validation_accepts_only_supported_agents() {
        assert!(is_valid_hook_source("codex"));
        assert!(is_valid_hook_source("claude-code"));
        assert!(is_valid_hook_source("copilot"));
        assert!(!is_valid_hook_source("unknown"));
        assert!(!is_valid_hook_source(""));
    }

    #[test]
    fn cli_info_reports_package_name() {
        assert!(cli_info().starts_with("agent-pets "));
    }

    #[test]
    fn read_agent_pets_port_at_rejects_missing_and_invalid_files() {
        let missing = env::temp_dir().join("agent-pets-missing-port-for-test");
        let invalid =
            env::temp_dir().join(format!("agent-pets-invalid-port-{}", std::process::id()));

        let _ = fs::remove_file(&missing);
        fs::write(&invalid, "not-a-port").unwrap();

        assert_eq!(read_agent_pets_port_at(&missing), None);
        assert_eq!(read_agent_pets_port_at(&invalid), None);

        let _ = fs::remove_file(&invalid);
    }

    #[test]
    fn read_agent_pets_port_at_reads_valid_port() {
        let path = env::temp_dir().join(format!("agent-pets-valid-port-{}", std::process::id()));
        fs::write(&path, "34567\n").unwrap();

        assert_eq!(read_agent_pets_port_at(&path), Some(34567));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn hook_command_uses_installed_cli_path_not_curl() {
        let path = PathBuf::from("/Users/example/.agent-pets/bin/agent-pets");
        let command = hook_command("codex", &path);

        assert_eq!(
            command,
            "'/Users/example/.agent-pets/bin/agent-pets' hook codex"
        );
        assert!(!command.contains("curl"));
    }

    #[test]
    fn hook_command_quotes_spaces() {
        let path = PathBuf::from("/Users/example/My Tools/agent-pets");
        assert_eq!(
            hook_command("copilot", &path),
            "'/Users/example/My Tools/agent-pets' hook copilot"
        );
    }

    #[test]
    fn remove_agent_pets_codex_hooks_new_format() {
        let mut hooks = json!({
            "hooks": {
                "UserPromptSubmit": [
                    {"hooks": [{"type": "command", "command": "agent-pets hook codex", "timeout": 1}]},
                    {"hooks": [{"type": "command", "command": "echo keep", "timeout": 2}]}
                ],
                "Stop": [
                    {"hooks": [{"type": "command", "command": "agent-pets hook codex", "timeout": 1}]}
                ]
            }
        });

        assert_eq!(remove_agent_pets_hooks_from_codex(&mut hooks), 2);

        assert_eq!(
            hooks,
            json!({
                "hooks": {
                    "UserPromptSubmit": [
                        {"hooks": [{"type": "command", "command": "echo keep", "timeout": 2}]}
                    ],
                    "Stop": []
                }
            })
        );
    }

    #[test]
    fn remove_agent_pets_codex_hooks_old_format_migration() {
        let mut hooks = json!({
            "UserPromptSubmit": [
                {"type": "command", "command": "agent-pets hook codex", "timeout": 2},
                {"type": "command", "command": "echo keep", "timeout": 2}
            ],
            "Stop": [
                {"type": "command", "command": "p=$(cat ~/.agent-pets/port) && curl -s http://127.0.0.1:$p/events/codex; exit 0"}
            ]
        });

        assert_eq!(remove_agent_pets_hooks_from_codex(&mut hooks), 2);

        assert_eq!(
            hooks,
            json!({
                "UserPromptSubmit": [
                    {"type": "command", "command": "echo keep", "timeout": 2}
                ],
                "Stop": []
            })
        );
    }

    #[test]
    fn remove_agent_pets_claude_hooks_keeps_non_agent_pets_handlers_in_group() {
        let mut settings = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "",
                        "hooks": [
                            {"type": "command", "command": "agent-pets hook claude-code"},
                            {"type": "command", "command": "echo keep"}
                        ]
                    },
                    {
                        "matcher": "Bash",
                        "hooks": [{"type": "command", "command": "echo also-keep"}]
                    }
                ],
                "Stop": [
                    {"type": "command", "command": "p=$(cat ~/.agent-pets/port) && curl -s http://127.0.0.1:$p/events/claude-code; exit 0"}
                ]
            }
        });

        assert_eq!(
            remove_agent_pets_hooks_from_claude_settings(&mut settings),
            2
        );

        assert_eq!(
            settings,
            json!({
                "hooks": {
                    "PreToolUse": [
                        {
                            "matcher": "",
                            "hooks": [
                                {"type": "command", "command": "echo keep"}
                            ]
                        },
                        {
                            "matcher": "Bash",
                            "hooks": [{"type": "command", "command": "echo also-keep"}]
                        }
                    ],
                    "Stop": []
                }
            })
        );
    }

    #[test]
    fn remove_agent_pets_claude_hooks_counts_nested_handlers_once() {
        let mut settings = json!({
            "hooks": {
                "Stop": [
                    {
                        "matcher": "",
                        "hooks": [
                            {"type": "command", "command": "agent-pets hook claude-code"}
                        ]
                    }
                ]
            }
        });

        assert_eq!(
            remove_agent_pets_hooks_from_claude_settings(&mut settings),
            1
        );
        assert_eq!(settings, json!({"hooks": {"Stop": []}}));
    }

    #[test]
    fn copilot_agent_pets_hook_file_is_safe_to_remove() {
        let config = json!({
            "version": 1,
            "hooks": {
                "Stop": [{"bash": "agent-pets hook copilot", "timeoutSec": 2}]
            }
        });

        assert!(is_agent_pets_copilot_config(&config));
        assert!(!is_agent_pets_copilot_config(&json!({
            "version": 1,
            "hooks": {
                "Stop": [{"bash": "echo keep", "timeoutSec": 2}]
            }
        })));
    }

    #[test]
    fn tray_menu_action_ignores_unknown_items() {
        assert_eq!(TrayMenuAction::from_id("agent-pets-title"), None);
        assert_eq!(TrayMenuAction::from_id("missing"), None);
    }
}
