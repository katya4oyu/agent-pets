# Hook Integrations

See [Hook Schema Research](./hook-schema-research.md) for the official source
URLs, event lists, payload fields, and normalization rules behind this design.

Agent Pets should use one installed CLI command across supported agents:

```text
~/.agent-pets/bin/agent-pets hook <source>
```

The command pipes hook JSON from stdin directly to the resident desktop app. The
desktop app normalizes each agent payload server-side, emits an Agent Pets event,
and the hook command exits quickly. The command should avoid stdout and return
success when the desktop app is unavailable so the agent workflow is never
blocked by the companion.

Do not require the lifecycle event name as a command argument. Codex and Claude
Code provide `hook_event_name`. GitHub Copilot has two documented payload
formats, so the server-side normalizer must also handle Copilot's camelCase
payloads that do not include an event name:

- Codex includes `hook_event_name`.
- Claude Code includes `hook_event_name`.
- GitHub Copilot includes `hook_event_name` when hooks are configured with
  PascalCase event keys such as `PreToolUse` instead of camelCase keys such as
  `preToolUse`.
- GitHub Copilot camelCase payloads omit `hook_event_name`; infer the event from
  the documented payload shape.

The source path segment is useful because the surrounding payload schemas and
tool names differ between agents. Event routing should be based on stdin first:

```text
source = path segment after /events/
event = payload.hook_event_name
tool = payload.tool_name
cwd = payload.cwd
session = payload.session_id
```

## Transport and performance

The first transport should be a localhost HTTP POST to the resident desktop app:

```text
POST http://127.0.0.1:<port>/events/<source>
```

HTTP remains an internal localhost transport between the CLI and desktop app.
A Unix domain socket can be revisited later if port discovery or local access
control becomes painful.

The hook command must never make the coding agent feel slower:

- Send stdin once with one POST, then exit.
- Use a very small connect/request timeout, around `100-250ms`.
- Exit `0` when the app is not running, the port file is missing, JSON parsing
  fails, or the POST times out.
- Do not write stdout by default. Some agents treat stdout as control data or
  model-visible context for particular events.
- Prefer async hook configuration where the agent supports it and still provides
  stdin to the command. If async hooks are unsupported or skipped for an agent,
  keep the synchronous command tiny and timeout-bound.

The desktop app should treat hook events as lossy status updates. Missing one
event is better than delaying or blocking the coding agent.

## Codex

Shared repository config can live in:

- `<repo>/.codex/hooks.json`
- `<repo>/.codex/config.toml`

User config can live in:

- `~/.codex/hooks.json`
- `~/.codex/config.toml`

Useful events for the first adapter are `SessionStart`, `UserPromptSubmit`,
`PreToolUse`, `PermissionRequest`, `PostToolUse`, and `Stop`.

Recommended command shape:

```json
{
  "type": "command",
  "command": "'/Users/example/.agent-pets/bin/agent-pets' hook codex",
  "timeout": 1
}
```

## Claude Code

Shared repository config can live in:

- `<repo>/.claude/settings.json`

Local or user config can live in:

- `<repo>/.claude/settings.local.json`
- `~/.claude/settings.json`

Use `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`,
`PostToolUseFailure`, `Notification`, and `Stop` for a practical first pass.
`Notification` events are especially useful for permission and idle prompts.

Claude hooks can use non-zero exits for control flow, so the notification
adapter should exit `0` and suppress output unless a future blocking integration
intentionally needs different behavior.

## GitHub Copilot CLI

Repository hooks can live in:

- `<repo>/.github/hooks/*.json`

User hooks can live in:

- `~/.copilot/hooks/*.json`
- `$COPILOT_HOME/hooks/*.json`

Inline settings can live in:

- `<repo>/.github/copilot/settings.json`
- `<repo>/.github/copilot/settings.local.json`
- `~/.copilot/settings.json`

Use lifecycle events such as `sessionStart`, `userPromptSubmitted`,
`preToolUse`, `permissionRequest`, `postToolUse`, `postToolUseFailure`,
`notification`, `agentStop`, `sessionEnd`, and `errorOccurred`.

Copilot supports both camelCase event keys and PascalCase VS Code compatible
event keys. Agent Pets must parse both documented payload formats.

Agent Pets setup should target local Copilot CLI user hooks first.

## Normalized State Mapping

| Incoming event | Agent Pets state | Suggested label |
| --- | --- | --- |
| prompt submitted | `thinking` | `Thinking` |
| pre tool use, shell | `running` | `Running shell` |
| pre tool use, edit | `editing` | `Editing` |
| permission request | `waiting_approval` | `Waiting approval` |
| notification, idle or prompt | `waiting_approval` | `Needs attention` |
| post tool failure or error | `error` | `Tool failed` |
| stop or session end | `done` | `Done` |
