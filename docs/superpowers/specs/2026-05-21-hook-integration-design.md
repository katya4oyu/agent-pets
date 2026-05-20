# Hook Integration Design

Date: 2026-05-21

## Overview

Implement end-to-end hook event handling for Agent Pets. Coding agents (Codex,
Claude Code, GitHub Copilot CLI) fire lifecycle hooks that invoke a small CLI
adapter. The adapter normalizes each agent's payload into a shared event schema
and POSTs it to a localhost HTTP server running inside the Tauri app. The app
updates the pet animation and speech bubble in response.

## Scope

**In:**
- HTTP event server inside Tauri backend (`tiny_http`, port file at `~/.agent-pets/port`)
- Hook adapter CLI binary (`agent-pets hook <source>`) supporting Codex, Claude Code, Copilot CLI
- Frontend animation row switching + speech bubble updates driven by Tauri events

**Out:**
- `agent-pets setup <agent>` config generation command
- Multi-pet / multi-session UI
- Unix domain socket transport

---

## Architecture

```
[Codex / Claude Code / Copilot CLI]
    │ lifecycle hook → stdin JSON
    ▼
agent-pets hook <source>           Rust binary ([[bin]] in Cargo.toml)
    │ POST /events  timeout 200ms
    ▼
Tauri app HTTP server              tiny_http on a free port
    │ port written to ~/.agent-pets/port on startup
    │ app_handle.emit("agent-state-changed", payload)
    ▼
Frontend (main.ts)
    │ listen("agent-state-changed", …)
    │ state → animation row + speech bubble update
    ▼
Pet sprite animation + speech bubble
```

---

## Components

### 1. Tauri Backend — HTTP Event Server (`lib.rs`)

On `setup`:
1. Bind `tiny_http::Server` to `127.0.0.1:0` (OS assigns free port).
2. Write the bound port number to `~/.agent-pets/port`.
3. Spawn a background thread: loop over incoming requests, parse JSON body as
   `HookEvent`, update shared `Arc<Mutex<AppState>>`, emit Tauri event
   `agent-state-changed` via `AppHandle`.
4. On app exit (via `on_window_event` or `RunEvent::Exit`), remove the port file.

**Key types:**

```rust
// Incoming normalized event body
struct HookEvent {
    source: String,       // "codex" | "claude-code" | "copilot"
    state: AgentState,
    label: String,        // e.g. "Running shell"
    message: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
}

enum AgentState {
    Thinking,
    Running,
    Editing,
    WaitingApproval,
    Done,
    Error,
}
```

Emit payload mirrors `HookEvent` serialized to JSON.

**Error handling:**
- JSON parse failure → respond 400, continue loop (no crash).
- `emit` failure → log, continue loop.
- Port file write failure → log warning, continue (adapter will get connection refused and exit 0).

---

### 2. Hook Adapter CLI (`src/bin/agent_pets.rs`)

Invoked as: `agent-pets hook <source>`

**Algorithm:**
1. Parse `source` from `argv[1]` (`hook`) and `argv[2]` (`codex` / `claude-code` / `copilot`).
2. Read all of stdin to a string (non-blocking read with short deadline).
3. Parse JSON best-effort. If parse fails → exit 0.
4. Normalize payload to `HookEvent` using source-specific logic (see below).
5. Read `~/.agent-pets/port`. If missing or not a valid port → exit 0.
6. POST JSON to `http://127.0.0.1:<port>/events` with a 200ms connect+request timeout.
7. Exit 0 regardless of HTTP result.

**stdout**: never written. **stderr**: silent (agents may surface stderr to users).

**Normalization — event name extraction:**

| Source | Primary field | Fallback |
|---|---|---|
| codex | `hook_event_name` | — |
| claude-code | `hook_event_name` | — |
| copilot | `hook_event_name` (PascalCase config) | camelCase key detection |

**Normalization — state mapping:**

| Incoming event / tool | state | label |
|---|---|---|
| `UserPromptSubmit` / `userPromptSubmitted` | `thinking` | Thinking |
| `PreToolUse` + tool is Bash/bash/shell | `running` | Running shell |
| `PreToolUse` + tool is Write/Edit/apply_patch/edit/create | `editing` | Editing |
| `PreToolUse` + other tool | `running` | Using tool |
| `PermissionRequest` / `Elicitation` | `waiting_approval` | Waiting approval |
| `Notification` (any type) | `waiting_approval` | Needs attention |
| `PostToolUse` (success) | `running` | Tool completed |
| `PostToolUseFailure` / `ErrorOccurred` / `PermissionDenied` | `error` | Tool failed |
| `Stop` / `SessionEnd` / `AgentStop` / `SubagentStop` | `done` | Done |
| `SessionStart` | `done` | Ready |

**Session and location fields:**

| Normalized | Codex | Claude Code | Copilot PascalCase | Copilot camelCase |
|---|---|---|---|---|
| `session_id` | `session_id` | `session_id` | `session_id` | `sessionId` |
| `cwd` | `cwd` | `cwd` | `cwd` | `cwd` |
| `message` | `prompt` / `tool_input.command` / `tool_input.file_path` | same | same | `toolArgs` preview |
| `timestamp` | adapter generates | adapter generates | `timestamp` | convert from ms |

**Cargo dependency:** `ureq` (sync, tiny, no extra runtime). Configured with
`timeout(Duration::from_millis(200))`.

---

### 3. Frontend — Animation + Speech Bubble (`main.ts`)

**Animation system refactor:**

Replace the current single-row idle loop with a stateful animator:

```ts
interface AnimationSpec {
  row: number;
  frameCount: number;
  durations: number[];  // per-frame ms, last frame repeats if shorter than frameCount
}

// State machine: cancels current loop, starts new row immediately
function setAnimation(spec: AnimationSpec): void
```

**State → animation row mapping:**

| state | row | animation name |
|---|---|---|
| `thinking` | 8 | review |
| `running` | 7 | running |
| `editing` | 8 | review |
| `waiting_approval` | 3 | waving |
| `done` | 0 | idle |
| `error` | 5 | failed |

**Row durations (from spritesheet doc):**
- Row 0 idle: `[280,110,110,140,140,320]` (6 frames)
- Row 3 waving: `[140,140,140,280]` (4 frames)
- Row 5 failed: `[140,140,140,140,140,140,140,240]` (8 frames)
- Row 7 running: `[120,120,120,120,120,220]` (6 frames)
- Row 8 review: `[150,150,150,150,150,280]` (6 frames)

**Tauri event listener:**

```ts
import { listen } from "@tauri-apps/api/event";

await listen<HookEvent>("agent-state-changed", (event) => {
  applyState(event.payload);
});
```

`applyState` updates:
- `setAnimation(rowSpecForState(event.payload.state))`
- `speechTitle.textContent = event.payload.label`
- `message.textContent = event.payload.message ?? stateLabels[event.payload.state]`

**Reduced motion:** if `prefers-reduced-motion: reduce`, always draw column 0 of the current row (static frame).

---

## Cargo.toml Changes

```toml
[dependencies]
# existing…
tiny_http = "0.12"

[[bin]]
name = "agent-pets"
path = "src/bin/agent_pets.rs"
```

The CLI binary does NOT depend on Tauri — only on `serde_json`, `ureq`, and std.

---

## File Changes Summary

| File | Change |
|---|---|
| `app/src-tauri/Cargo.toml` | Add `tiny_http`, `ureq`; add `[[bin]]` |
| `app/src-tauri/src/lib.rs` | HTTP server setup, port file mgmt, Tauri event emit |
| `app/src-tauri/src/bin/agent_pets.rs` | New — hook adapter CLI |
| `app/src/main.ts` | Animation refactor, Tauri event listener |

---

## Error Handling Summary

| Scenario | Behavior |
|---|---|
| App not running | CLI reads no port file → exit 0 |
| Port file exists but app crashed | HTTP connect fails within 200ms → exit 0 |
| Malformed stdin JSON | CLI exits 0 silently |
| POST timeout | exit 0 |
| Tauri event emit fails | Rust logs, HTTP server loop continues |
| Unknown event name | CLI maps to `done` / skips (no POST if truly unrecognized) |
| Frontend receives unknown state | keeps current animation |

---

## Manual Setup Instructions (until `setup` command exists)

### Claude Code (`~/.claude/settings.json`)
```json
{
  "hooks": {
    "UserPromptSubmit": [{"type": "command", "command": "/path/to/agent-pets hook claude-code", "async": true}],
    "PreToolUse":       [{"type": "command", "command": "/path/to/agent-pets hook claude-code", "async": true}],
    "PermissionRequest":[{"type": "command", "command": "/path/to/agent-pets hook claude-code", "async": true}],
    "PostToolUse":      [{"type": "command", "command": "/path/to/agent-pets hook claude-code", "async": true}],
    "PostToolUseFailure":[{"type":"command","command":"/path/to/agent-pets hook claude-code","async":true}],
    "Notification":     [{"type": "command", "command": "/path/to/agent-pets hook claude-code", "async": true}],
    "Stop":             [{"type": "command", "command": "/path/to/agent-pets hook claude-code", "async": true}]
  }
}
```

### Codex (`~/.codex/hooks.json`)
```json
{
  "UserPromptSubmit":  [{"type":"command","command":"/path/to/agent-pets hook codex","timeout":1}],
  "PreToolUse":        [{"type":"command","command":"/path/to/agent-pets hook codex","timeout":1}],
  "PermissionRequest": [{"type":"command","command":"/path/to/agent-pets hook codex","timeout":1}],
  "PostToolUse":       [{"type":"command","command":"/path/to/agent-pets hook codex","timeout":1}],
  "Stop":              [{"type":"command","command":"/path/to/agent-pets hook codex","timeout":1}]
}
```

### Copilot CLI (`~/.copilot/hooks/agent-pets.json`)
```json
{
  "version": 1,
  "hooks": {
    "UserPromptSubmit":   [{"bash": "/path/to/agent-pets hook copilot", "timeoutSec": 1}],
    "PreToolUse":         [{"bash": "/path/to/agent-pets hook copilot", "timeoutSec": 1}],
    "PostToolUse":        [{"bash": "/path/to/agent-pets hook copilot", "timeoutSec": 1}],
    "PostToolUseFailure": [{"bash": "/path/to/agent-pets hook copilot", "timeoutSec": 1}],
    "Stop":               [{"bash": "/path/to/agent-pets hook copilot", "timeoutSec": 1}],
    "ErrorOccurred":      [{"bash": "/path/to/agent-pets hook copilot", "timeoutSec": 1}]
  }
}
```
