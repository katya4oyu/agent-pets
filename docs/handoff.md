# Handoff

## Product direction

Build a Codex-pets-like desktop resident companion that can show agent progress
and notifications while the user is working in other apps. The first version
does not need multi-pet support. It should show a single pet and a speech bubble
containing:

- A short message or progress summary.
- A compact state label such as `Thinking`, `Running Bash`, `Editing`,
  `Waiting approval`, or `Done`.

The visual target is close to Codex pets: a small floating overlay, pet sprite,
rounded speech bubble, and a spinner/status icon.

## Name

- Repository: `agent-pets`
- Product name: `Agent Pets`
- Hook command: `curl` to `http://127.0.0.1:<port>/events/<source>`
- Config directory: `~/.agent-pets`

## First implementation target

Use Codex Hooks as the first event source.

Codex discovers hooks from:

- `~/.codex/hooks.json`
- `~/.codex/config.toml`
- `<repo>/.codex/hooks.json`
- `<repo>/.codex/config.toml`

Useful events:

- `SessionStart`: app/session startup or resume.
- `UserPromptSubmit`: set bubble to `Thinking`.
- `PreToolUse`: set bubble to running/editing/tool status.
- `PermissionRequest`: set bubble to approval waiting.
- `PostToolUse`: update with tool completion or failure.
- `Stop`: set bubble to done.

Hook adapters should be quick and side-effect-only. Avoid writing stdout unless
the Codex hook event expects it. `Stop` expects JSON on stdout if stdout is used,
so the safest default is no stdout.

The hook command should pipe stdin directly to the desktop app. The desktop app
reads `hook_event_name` from the posted JSON and normalizes the event
server-side. The generated command for every Codex event can therefore be the
same:

```text
p=$(cat ~/.agent-pets/port 2>/dev/null) && curl -s --max-time 0.2 -X POST "http://127.0.0.1:$p/events/codex" -H 'Content-Type: application/json' -d @- 2>/dev/null; exit 0
```

The command should use a tiny HTTP timeout, around `100-250ms`, and exit `0` if
the app is unavailable. Agent Pets status updates are intentionally lossy; they
must not slow down or block the coding agent.

## Event API

The desktop app should expose a localhost-only endpoint:

```text
POST /events/<source>
```

Example body:

```json
{
  "source": "codex",
  "event": "tool_started",
  "state": "running",
  "title": "Running Bash",
  "message": "pnpm test",
  "severity": "info",
  "session_id": "abc123",
  "turn_id": "turn_123",
  "cwd": "/path/to/project",
  "timestamp": "2026-05-20T12:54:42Z"
}
```

## Later adapters

Claude Code:

- Use `Notification` hooks for permission prompts, idle prompts, and other
  attention requests.
- It provides fields such as `message`, `title`, `notification_type`,
  `session_id`, `transcript_path`, and `cwd`.

GitHub Copilot CLI:

- Use CLI notification hooks.
- Normalize system notification events into the same `/events` schema.
- Prefer PascalCase hook keys where supported so the payload includes
  `hook_event_name` and snake_case field names.

## Open decisions

- Whether the `agent-pets` CLI is a Rust binary in the Tauri workspace or a
  separate helper package. Rust is the current preference to avoid a Bun/Node
  runtime dependency.
- How the hook finds the current app port: fixed port, config file, or later
  Unix domain socket. Localhost HTTP is the first target.
- Whether the overlay should be click-through by default.
- Where to store bundled custom pet assets.
