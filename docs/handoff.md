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
- Hook binary: `agent-pets-hook`
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

## Event API

The desktop app should expose a localhost-only endpoint:

```text
POST /events
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

## Open decisions

- Whether the hook adapter is Rust, shell, Node, or bundled with the Tauri app.
- How the hook finds the current app port: fixed port, config file, or Unix
  domain socket.
- Whether the overlay should be click-through by default.
- Where to store bundled custom pet assets.
