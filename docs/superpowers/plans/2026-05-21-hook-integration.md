# Hook Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Receive lifecycle hook events from Codex, Claude Code, and GitHub Copilot CLI, update the pet animation row and speech bubble in the Tauri desktop overlay.

**Architecture:** A `tiny_http` server starts inside the Tauri app, writes its port to `~/.agent-pets/port`, and emits Tauri events (`agent-state-changed`) on valid POSTs. A self-contained Rust CLI binary (`agent-pets hook <source>`) reads stdin, normalizes the payload into a common schema, reads the port file, and POSTs to the server within a 200ms timeout. The frontend listens for `agent-state-changed` and switches the sprite animation row and speech bubble.

**Tech Stack:** Rust/Tauri v2, `tiny_http 0.12`, `ureq 2`, `serde_json`, TypeScript/Vite, `@tauri-apps/api` v2

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `app/src-tauri/Cargo.toml` | Modify | Add `tiny_http`, `ureq`; declare `[[bin]]` |
| `app/src-tauri/src/lib.rs` | Modify | HTTP server, port file management, Tauri event emission |
| `app/src-tauri/src/bin/agent_pets.rs` | **Create** | Self-contained hook adapter CLI |
| `app/src/main.ts` | Modify | Multi-row animation, Tauri event listener |

---

## Task 1: Cargo dependencies and binary target

**Files:**
- Modify: `app/src-tauri/Cargo.toml`

- [ ] **Step 1: Add dependencies and [[bin]] to Cargo.toml**

Replace the existing `[dependencies]` block and append `[[bin]]`:

```toml
[dependencies]
tauri = { version = "2", features = ["macos-private-api", "tray-icon"] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tiny_http = "0.12"
ureq = "2"

[[bin]]
name = "agent-pets"
path = "src/bin/agent_pets.rs"
```

- [ ] **Step 2: Verify Cargo resolves without error**

```bash
cd app/src-tauri && cargo fetch
```

Expected: No errors. `Cargo.lock` updated with `tiny_http` and `ureq` entries.

- [ ] **Step 3: Commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock
git commit -m "chore: add tiny_http and ureq dependencies, declare agent-pets bin"
```

---

## Task 2: Shared event types in lib.rs

**Files:**
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Add HookEvent types at the top of lib.rs**

After the existing `use` statements, insert these type definitions (before `PetAsset`):

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Thinking,
    Running,
    Editing,
    WaitingApproval,
    Done,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HookEvent {
    pub source: String,
    pub state: AgentState,
    pub label: String,
    pub message: Option<String>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub timestamp: Option<String>,
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd app/src-tauri && cargo check
```

Expected: no errors.

---

## Task 3: HTTP event server in lib.rs

**Files:**
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Add port file helpers after the HookEvent types**

```rust
fn agent_pets_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".agent-pets"))
}

fn port_file_path() -> Option<std::path::PathBuf> {
    agent_pets_dir().map(|d| d.join("port"))
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
```

- [ ] **Step 2: Add start_event_server function**

```rust
fn start_event_server(app_handle: tauri::AppHandle) {
    use std::io::Read;
    use tauri::Emitter;

    std::thread::spawn(move || {
        // Probe for a free port, then bind tiny_http to it.
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
            if request.method() != &tiny_http::Method::Post || request.url() != "/events" {
                let _ = request.respond(
                    tiny_http::Response::from_string("not found").with_status_code(404),
                );
                continue;
            }

            let mut body = String::new();
            let read_ok = request.as_reader().read_to_string(&mut body).is_ok();

            if !read_ok {
                let _ = request.respond(
                    tiny_http::Response::from_string("bad request").with_status_code(400),
                );
                continue;
            }

            match serde_json::from_str::<HookEvent>(&body) {
                Ok(event) => {
                    let _ = app_handle.emit("agent-state-changed", &event);
                    let _ = request.respond(tiny_http::Response::from_string("ok"));
                }
                Err(_) => {
                    let _ = request.respond(
                        tiny_http::Response::from_string("bad request").with_status_code(400),
                    );
                }
            }
        }
    });
}
```

- [ ] **Step 3: Replace the run() function**

Replace the existing `pub fn run()` with this version that hooks into setup and exit:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            start_event_server(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![load_pet_asset, ping])
        .build(tauri::generate_context!())
        .expect("error building Agent Pets")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                remove_port_file();
            }
        });
}
```

- [ ] **Step 4: Verify lib compiles**

```bash
cd app/src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/lib.rs
git commit -m "feat: add HTTP event server and port file management to Tauri backend"
```

---

## Task 4: Hook adapter CLI — normalization (TDD)

**Files:**
- Create: `app/src-tauri/src/bin/agent_pets.rs`

The entire normalization logic lives here. Write tests first.

- [ ] **Step 1: Create the file with normalization functions and tests**

Create `app/src-tauri/src/bin/agent_pets.rs`:

```rust
use serde::{Deserialize, Serialize};
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

/// Returns (state, label) or None if the event should be skipped.
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

/// Normalizes an agent hook payload to NormalizedEvent.
/// Returns None if the event should be skipped (unknown event).
fn normalize(payload: &Value, source: &str) -> Option<NormalizedEvent> {
    // Extract event name
    let event_name = str_val(payload, "hook_event_name")?;

    // Extract tool name — try snake_case first, then camelCase (Copilot)
    let tool_name = str_val(payload, "tool_name")
        .or_else(|| str_val(payload, "toolName"));

    let (state, label) = map_state(&event_name, tool_name.as_deref())?;

    let message = extract_message(payload, &event_name, tool_name.as_deref());

    // Session / location — snake_case first, camelCase fallback for Copilot
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

    // --- map_state tests ---

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

    // --- normalize tests ---

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
        // Copilot camelCase format (no hook_event_name via PascalCase config)
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
```

- [ ] **Step 2: Run the tests to verify they pass**

```bash
cd app/src-tauri && cargo test --bin agent-pets 2>&1
```

Expected: All tests pass. Output ends with `test result: ok. N passed; 0 failed`.

- [ ] **Step 3: Build the CLI binary to verify it compiles**

```bash
cd app/src-tauri && cargo build --bin agent-pets 2>&1 | tail -5
```

Expected: `Finished ... target/debug/agent-pets`

- [ ] **Step 4: Smoke-test the adapter with fake stdin**

```bash
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test","prompt":"hello"}' \
  | app/src-tauri/target/debug/agent-pets hook claude-code
echo "exit code: $?"
```

Expected: exit code 0 (port file missing → silent exit).

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/bin/agent_pets.rs
git commit -m "feat: add hook adapter CLI with normalization for Codex, Claude Code, and Copilot"
```

---

## Task 5: Verify HTTP server and CLI end-to-end

**Files:** (read-only verification — no new files)

- [ ] **Step 1: Build the full Tauri library to confirm everything compiles together**

```bash
cd app/src-tauri && cargo build --lib 2>&1 | tail -10
```

Expected: Compiles without errors.

- [ ] **Step 2: Build the CLI binary with lib as dependency**

```bash
cd app/src-tauri && cargo build 2>&1 | tail -10
```

Expected: Both `agent-pets-app` (from main.rs) and `agent-pets` (from bin) compile.

- [ ] **Step 3: Manual integration test (requires Tauri dev server running)**

In a terminal, run the Tauri app:
```bash
cd app && pnpm tauri dev &
```

Wait 5 seconds for the app to start and write the port file, then:

```bash
PORT=$(cat ~/.agent-pets/port)
echo "App port: $PORT"

curl -s -X POST http://127.0.0.1:$PORT/events \
  -H "Content-Type: application/json" \
  -d '{"source":"test","state":"thinking","label":"Thinking","message":"Is it working?"}' \
  -o /dev/null -w "%{http_code}"
```

Expected: HTTP 200. The speech bubble in the app should not crash (frontend event listener not yet wired — that's Task 6+7).

- [ ] **Step 4: Test the CLI adapter against the running app**

```bash
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test","prompt":"hello world"}' \
  | app/src-tauri/target/debug/agent-pets hook claude-code
echo "exit code: $?"
```

Expected: exit code 0. No visible output.

---

## Task 6: Frontend — multi-row animation system

**Files:**
- Modify: `app/src/main.ts`

- [ ] **Step 1: Replace the animation constants and add AnimationSpec type**

In `main.ts`, after the existing `AgentState` type definition and before `stateLabels`, add:

```typescript
interface AnimationSpec {
  row: number;
  frameCount: number;
  durations: number[];
}

const animations: Record<AgentState, AnimationSpec> = {
  thinking:         { row: 8, frameCount: 6, durations: [150, 150, 150, 150, 150, 280] },
  running:          { row: 7, frameCount: 6, durations: [120, 120, 120, 120, 120, 220] },
  editing:          { row: 8, frameCount: 6, durations: [150, 150, 150, 150, 150, 280] },
  waiting_approval: { row: 3, frameCount: 4, durations: [140, 140, 140, 280] },
  done:             { row: 0, frameCount: 6, durations: [280, 110, 110, 140, 140, 320] },
  error:            { row: 5, frameCount: 8, durations: [140, 140, 140, 140, 140, 140, 140, 240] },
};
```

Remove the existing `idleDurations` constant (it's replaced by `animations.done.durations`).

- [ ] **Step 2: Add the animation controller**

After the `animations` object, add a module-level timer reference and the `setAnimation` function:

```typescript
let animationTimer: ReturnType<typeof setTimeout> | null = null;

function setAnimation(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  spec: AnimationSpec,
) {
  if (animationTimer !== null) {
    clearTimeout(animationTimer);
    animationTimer = null;
  }
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reducedMotion) {
    drawFrame(context, image, 0, spec.row);
    return;
  }
  let frame = 0;
  const tick = () => {
    drawFrame(context, image, frame, spec.row);
    const duration = spec.durations[frame] ?? spec.durations[spec.durations.length - 1] ?? 180;
    frame = (frame + 1) % spec.frameCount;
    animationTimer = setTimeout(tick, duration);
  };
  tick();
}
```

- [ ] **Step 3: Update loadMio to use setAnimation and expose context + image**

Replace the existing `loadMio` function with this version that stores context and image at module scope for use by the event listener:

```typescript
let spriteContext: CanvasRenderingContext2D | null = null;
let spriteImage: HTMLImageElement | null = null;

async function loadMio() {
  if (!sprite) return;
  const context = sprite.getContext("2d");
  if (!context) return;

  try {
    const asset = await invoke<PetAsset>("load_pet_asset", { petId: "mio" });
    const image = new Image();
    const objectUrl = bytesToObjectUrl(asset);
    image.src = objectUrl;
    image.onload = () => {
      spriteContext = context;
      spriteImage = image;
      setAnimation(context, image, animations.done);
      if (speechTitle) speechTitle.textContent = asset.displayName;
      if (message) message.textContent = "is ready and waiting for your next prompt.";
    };
  } catch (error) {
    console.error(error);
    if (speechTitle) speechTitle.textContent = stateLabels.error;
    if (message) message.textContent = "Could not load Mio from ~/.codex/pets.";
  }
}
```

Remove the old `frame`, `animate`, and `idleDurations` references from `loadMio`.

- [ ] **Step 4: Verify TypeScript compiles**

```bash
cd app && pnpm build 2>&1 | tail -10
```

Expected: no TypeScript errors. Build succeeds.

---

## Task 7: Frontend — Tauri event listener

**Files:**
- Modify: `app/src/main.ts`

- [ ] **Step 1: Add the listen import at the top of main.ts**

After the existing imports, add:

```typescript
import { listen } from "@tauri-apps/api/event";
```

- [ ] **Step 2: Add the HookEventPayload type**

After the `AgentState` type, add:

```typescript
interface HookEventPayload {
  source: string;
  state: AgentState;
  label: string;
  message?: string;
  session_id?: string;
  cwd?: string;
  timestamp?: string;
}
```

- [ ] **Step 3: Add applyAgentState function**

After the `setAnimation` function, add:

```typescript
function applyAgentState(payload: HookEventPayload) {
  const spec = animations[payload.state] ?? animations.done;
  if (spriteContext && spriteImage) {
    setAnimation(spriteContext, spriteImage, spec);
  }
  if (speechTitle) speechTitle.textContent = payload.label;
  if (message) {
    message.textContent =
      payload.message ?? stateLabels[payload.state] ?? payload.label;
  }
}
```

- [ ] **Step 4: Register the listener at the bottom of main.ts**

After the `loadMio()` call at the bottom of the file, add:

```typescript
listen<HookEventPayload>("agent-state-changed", (event) => {
  applyAgentState(event.payload);
}).catch(console.error);
```

- [ ] **Step 5: Verify TypeScript compiles cleanly**

```bash
cd app && pnpm build 2>&1 | tail -10
```

Expected: no TypeScript errors.

- [ ] **Step 6: End-to-end smoke test**

Start the Tauri dev app:
```bash
cd app && pnpm tauri dev
```

In another terminal, send a thinking event:
```bash
PORT=$(cat ~/.agent-pets/port)
curl -s -X POST http://127.0.0.1:$PORT/events \
  -H "Content-Type: application/json" \
  -d '{"source":"test","state":"thinking","label":"Thinking","message":"Planning next step…"}'
```

Expected: pet switches to row 8 (review animation), speech bubble shows "Thinking" as title and "Planning next step…" as message.

Send more states to verify:
```bash
# Running shell
curl -s -X POST http://127.0.0.1:$PORT/events \
  -H "Content-Type: application/json" \
  -d '{"source":"test","state":"running","label":"Running shell","message":"cargo build"}'

# Waiting approval
curl -s -X POST http://127.0.0.1:$PORT/events \
  -H "Content-Type: application/json" \
  -d '{"source":"test","state":"waiting_approval","label":"Waiting approval"}'

# Done
curl -s -X POST http://127.0.0.1:$PORT/events \
  -H "Content-Type: application/json" \
  -d '{"source":"test","state":"done","label":"Done"}'
```

- [ ] **Step 7: Commit**

```bash
git add app/src/main.ts
git commit -m "feat: add multi-row animation and Tauri event listener for hook state updates"
```

---

## Task 8: Wire up Claude Code hooks for real-world test

This task configures Claude Code to send actual hook events to Agent Pets.

- [ ] **Step 1: Build the release CLI binary**

```bash
cd app/src-tauri && cargo build --release --bin agent-pets
```

Expected: `target/release/agent-pets` created.

- [ ] **Step 2: Note the binary path**

```bash
echo $(pwd)/target/release/agent-pets
```

Copy this path. You will use it in the next step.

- [ ] **Step 3: Add Claude Code hook configuration**

Edit `~/.claude/settings.json` (create if missing). Replace `/BINARY_PATH` with the path from Step 2:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {"type": "command", "command": "/BINARY_PATH hook claude-code", "async": true}
    ],
    "PreToolUse": [
      {"type": "command", "command": "/BINARY_PATH hook claude-code", "async": true}
    ],
    "PermissionRequest": [
      {"type": "command", "command": "/BINARY_PATH hook claude-code", "async": true}
    ],
    "PostToolUse": [
      {"type": "command", "command": "/BINARY_PATH hook claude-code", "async": true}
    ],
    "PostToolUseFailure": [
      {"type": "command", "command": "/BINARY_PATH hook claude-code", "async": true}
    ],
    "Notification": [
      {"type": "command", "command": "/BINARY_PATH hook claude-code", "async": true}
    ],
    "Stop": [
      {"type": "command", "command": "/BINARY_PATH hook claude-code", "async": true}
    ]
  }
}
```

- [ ] **Step 4: Verify the hooks fire**

Start the Agent Pets app (`pnpm tauri dev`), then open a new Claude Code session and type a prompt. The pet should animate through states as Claude works.

- [ ] **Step 5: Commit final state**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock \
        app/src-tauri/src/lib.rs app/src-tauri/src/bin/agent_pets.rs \
        app/src/main.ts
git commit -m "feat: complete hook integration — HTTP server, CLI adapter, animated states"
```
