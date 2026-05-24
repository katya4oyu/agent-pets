use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, fs, path::PathBuf, sync::Mutex};
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
    pub timestamp: Option<String>,
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

pub fn is_legacy_hook_invocation(args: &[String]) -> bool {
    args.get(1).is_some_and(|arg| arg == "hook")
}

// ── Normalization types ───────────────────────────────────────────────────────

struct HookInput {
    event_name: String,
    tool_name: Option<String>,
    tool_input: Option<Value>,
    message: Option<String>,
    error_message: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
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
        "Stop" | "AgentStop" | "agentStop" | "SessionEnd" | "sessionEnd" | "SubagentStop"
        | "subagentStop" => Some((AgentState::Done, "Done")),
        "SessionStart" | "sessionStart" => Some((AgentState::Done, "Ready")),
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
    let (state, label) = map_state(&input.event_name, input.tool_name.as_deref())?;
    let message = match input.event_name.as_str() {
        "UserPromptSubmit" | "userPromptSubmitted" => input.message,
        "Notification" | "notification" => input.message,
        "PreToolUse" | "preToolUse" => {
            extract_tool_message(input.tool_input.as_ref(), input.tool_name.as_deref())
        }
        "PostToolUseFailure" | "postToolUseFailure" | "ErrorOccurred" | "errorOccurred" => {
            input.error_message
        }
        _ => None,
    };
    Some(HookEvent {
        source: source.to_string(),
        state,
        label: label.to_string(),
        message,
        session_id: input.session_id,
        cwd: input.cwd,
        timestamp: input.timestamp,
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
        session_id: payload.session_id,
        cwd: payload.cwd,
        timestamp: payload.timestamp,
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
        session_id: payload.session_id,
        cwd: payload.cwd,
        timestamp: payload.timestamp.map(|ts| ts.to_string()),
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

fn upsert_event_hooks(hooks_obj: &mut serde_json::Map<String, Value>, event: &str, entry: Value) {
    let arr = hooks_obj
        .entry(event.to_string())
        .or_insert_with(|| Value::Array(vec![]));
    if let Some(arr) = arr.as_array_mut() {
        arr.retain(|e| {
            !e.get("command")
                .and_then(Value::as_str)
                .map_or(false, |c| c.contains("agent-pets"))
        });
        arr.push(entry);
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
            "hooks": [{"type": "command", "command": cmd}]
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

pub fn remove_agent_pets_hooks_from_codex(hooks: &mut Value) -> usize {
    let Some(hooks_obj) = hooks.as_object_mut() else {
        return 0;
    };
    hooks_obj
        .values_mut()
        .filter_map(Value::as_array_mut)
        .map(remove_agent_pets_from_hook_array)
        .sum()
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

    let mut hooks = read_json_or_empty(&path)?;
    {
        let hooks_obj = hooks
            .as_object_mut()
            .ok_or("hooks.json はオブジェクトではありません")?;

        for event in [
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PermissionRequest",
            "PostToolUse",
            "Stop",
        ] {
            upsert_event_hooks(
                hooks_obj,
                event,
                serde_json::json!({"type": "command", "command": cmd, "timeout": 2}),
            );
        }
    }

    write_json_atomic(&path, &hooks)?;
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
            serde_json::json!([{"bash": cmd, "timeoutSec": 2}]),
        );
    }
    let config = serde_json::json!({"version": 1, "hooks": hooks_obj});

    write_json_atomic(&path, &config)?;
    Ok(format!("Copilot: {}", path.display()))
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn setup_hooks(agent: String) -> Result<String, String> {
    let home = home_dir()?;
    let port_path = home.join(".agent-pets").join("port");
    let port_path_str = port_path.display().to_string();

    let make_cmd = |source: &str| {
        format!(
            "p=$(cat {port_path_str} 2>/dev/null) && \
             curl -s --max-time 0.2 -X POST \
             \"http://127.0.0.1:$p/events/{source}\" \
             -H 'Content-Type: application/json' -d @- 2>/dev/null; exit 0"
        )
    };

    let mut messages: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
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
        timestamp: None,
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
        .invoke_handler(tauri::generate_handler![load_pet_asset, ping, setup_hooks])
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
    fn legacy_hook_invocation_is_ignored() {
        let args = vec![
            "agent-pets".to_string(),
            "hook".to_string(),
            "claude-code".to_string(),
        ];
        assert!(is_legacy_hook_invocation(&args));
        assert!(!is_legacy_hook_invocation(&["agent-pets".to_string()]));
    }

    #[test]
    fn remove_agent_pets_codex_hooks_keeps_unrelated_commands() {
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
