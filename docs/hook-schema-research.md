# Hook Schema Research

This document records the source URLs and schema findings for Agent Pets hook
integration. It is intentionally source-oriented so implementation decisions can
be checked against upstream docs later.

## Sources Checked

- Codex Hooks: https://developers.openai.com/codex/hooks
- Claude Code Hooks reference: https://code.claude.com/docs/en/hooks
- GitHub Copilot hooks reference: https://docs.github.com/en/enterprise-cloud@latest/copilot/reference/hooks-reference
- GitHub Copilot CLI hooks guide: https://docs.github.com/en/enterprise-cloud@latest/copilot/how-tos/copilot-cli/customize-copilot/use-hooks

## Design Conclusions

- The adapter command should be `agent-pets hook <source>`, not
  `agent-pets hook <source> <event>`.
- The event should be read from the hook payload, primarily `hook_event_name`.
- Keep `<source>` as a CLI argument because Codex, Claude Code, and Copilot use
  different field names, tool names, and output semantics.
- Prefer generated user-level hook config for all repos:
  - Codex: `~/.codex/hooks.json` or `~/.codex/config.toml`
  - Claude Code: `~/.claude/settings.json`
  - Copilot CLI: `~/.copilot/hooks/agent-pets.json` or
    `$COPILOT_HOME/hooks/agent-pets.json`
- Use localhost HTTP from the adapter to the desktop app for the first
  implementation, but keep the adapter lossy and timeout-bound.
- Copilot has two documented payload formats. Agent Pets must parse both:
  camelCase payloads from camelCase event keys and VS Code compatible snake_case
  payloads from PascalCase event keys.

## Transport Notes

Agent Pets should not use upstream HTTP hooks directly as the default
integration, even where an agent supports them. A small local command adapter is
more portable across Codex, Claude Code, and Copilot, and it lets Agent Pets
hide port discovery, app-not-running behavior, payload normalization, and
timeouts in one place.

Recommended adapter behavior:

1. Read stdin once.
2. Parse JSON best-effort.
3. Normalize source-specific fields into the Agent Pets event shape.
4. POST one event to `http://127.0.0.1:<port>/events`.
5. Use an internal connect/request timeout around `100-250ms`.
6. Never write stdout by default.
7. Exit `0` on app unavailable, port unavailable, parse failure, timeout, or
   HTTP failure.

The desktop app should treat events as lossy status updates. Missing an Agent
Pets update is acceptable; delaying a coding agent is not.

## Codex

Source: https://developers.openai.com/codex/hooks

### Config and Runtime

- Hooks are enabled by default and can be disabled with `[features].hooks = false`.
- Codex discovers hooks from `hooks.json` and inline `[hooks]` tables in active
  config layers.
- Useful locations are `~/.codex/hooks.json`, `~/.codex/config.toml`,
  `<repo>/.codex/hooks.json`, and `<repo>/.codex/config.toml`.
- Hooks from multiple files all run; higher-precedence layers do not replace
  lower-precedence hooks.
- Multiple matching command hooks for the same event are launched concurrently.
- `timeout` is in seconds. If omitted, Codex uses `600`.
- `async` is parsed but async command hooks are not supported yet; handlers with
  `async: true` are skipped.
- Only `type: "command"` handlers run today.
- Commands run with the session `cwd`.

### Codex Common Input Fields

Every command hook receives one JSON object on stdin.

| Field | Type | Notes |
| --- | --- | --- |
| `session_id` | `string` | Current session or thread id. |
| `transcript_path` | `string` or `null` | Transcript path if available; not a stable interface. |
| `cwd` | `string` | Working directory for the session. |
| `hook_event_name` | `string` | Current hook event name. |
| `model` | `string` | Codex-specific active model slug. |
| `permission_mode` | `string` | Present for `SessionStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `UserPromptSubmit`, and `Stop`; values include `default`, `acceptEdits`, `plan`, `dontAsk`, and `bypassPermissions`. |

### Codex Events and Inputs

| Event | Matcher | Extra input fields | Agent Pets state |
| --- | --- | --- | --- |
| `SessionStart` | `source` | `source` with values such as `startup`, `resume`, or `clear` | `done` |
| `UserPromptSubmit` | ignored | `turn_id`, `prompt` | `thinking` |
| `PreToolUse` | `tool_name` plus aliases | `turn_id`, `tool_name`, `tool_use_id`, `tool_input` | `running` or `editing` |
| `PermissionRequest` | `tool_name` plus aliases | `turn_id`, `tool_name`, `tool_input`, optional `tool_input.description` | `waiting_approval` |
| `PostToolUse` | `tool_name` plus aliases | `turn_id`, `tool_name`, `tool_use_id`, `tool_input`, `tool_response` | `running`, `done`, or `error` based on result |
| `Stop` | ignored | `turn_id`, `stop_hook_active`, `last_assistant_message` | `done` |

Codex tool names currently include canonical values such as `Bash`,
`apply_patch`, and MCP names like `mcp__server__tool`. For `apply_patch`,
matchers may also use `Edit` or `Write`, but the payload still reports
`tool_name: "apply_patch"`.

### Codex Output Rules Relevant to Agent Pets

Agent Pets is observational and should not use stdout for control. It should
emit no stdout and return `0`.

Important upstream behavior:

- `SessionStart`, `UserPromptSubmit`, and `Stop` support common JSON output
  fields such as `continue`, `stopReason`, `systemMessage`, and
  `suppressOutput`.
- `PreToolUse` and `PermissionRequest` ignore plain stdout.
- `Stop` expects JSON on stdout if stdout is used; plain text is invalid.
- `PreToolUse` can deny or rewrite supported tool calls. Agent Pets must not do
  that unless a future explicit policy feature is added.
- `PostToolUse` cannot undo side effects; it can add context or block normal
  processing of the original tool result.

## Claude Code

Source: https://code.claude.com/docs/en/hooks

### Config and Runtime

- Hook settings can live in `~/.claude/settings.json`,
  `.claude/settings.json`, `.claude/settings.local.json`, managed policy
  settings, plugin `hooks/hooks.json`, or skill/agent frontmatter.
- Hook configuration has three levels: event, matcher group, handler.
- Command hooks receive JSON on stdin. HTTP hooks receive the same JSON as the
  POST body.
- Hook handler types include `command`, `http`, `mcp_tool`, `prompt`, and
  `agent`.
- Common handler fields include `type`, optional `if`, optional `timeout`,
  optional `statusMessage`, and optional `once`.
- Command hooks support `command`, optional `args`, and optional `async`.
- If `async: true`, Claude Code starts the hook process and immediately
  continues; the hook still receives the same stdin JSON.
- HTTP hook connection failures, non-2xx responses, and timeouts produce
  non-blocking errors and allow execution to continue.

### Claude Code Common Input Fields

Claude Code common fields are present across the hook schemas shown in the
reference examples.

| Field | Type | Notes |
| --- | --- | --- |
| `session_id` | `string` | Session id. |
| `transcript_path` | `string` | JSONL transcript path. |
| `cwd` | `string` | Current working directory. |
| `permission_mode` | `string` | Present for many turn/tool events. |
| `hook_event_name` | `string` | Event name. |
| `agent_id` | `string` | Present only inside a subagent call. |
| `effort` | object | Present for events inside a tool-use context when supported by the model. |

### Claude Code Events and Inputs

Claude Code exposes more events than Agent Pets needs at first. The table keeps
the full event list from the reference and marks the fields Agent Pets should
care about.

| Event | Matcher | Extra input fields | Agent Pets use |
| --- | --- | --- | --- |
| `SessionStart` | start source | `source` | set ready/resumed |
| `Setup` | setup trigger | `trigger` | ignore initially |
| `UserPromptSubmit` | none | `prompt` | `thinking` |
| `UserPromptExpansion` | command name | `expansion_type`, `command_name`, `command_args`, `command_source`, `prompt` | optional prompt status |
| `PreToolUse` | `tool_name` | `tool_name`, `tool_input`, `tool_use_id` | `running` or `editing` |
| `PermissionRequest` | `tool_name` | `tool_name`, `tool_input`, `permission_suggestions` | `waiting_approval` |
| `PermissionDenied` | `tool_name` | `tool_name`, `tool_input`, `tool_use_id`, `reason` | `error` or `waiting_approval` |
| `PostToolUse` | `tool_name` | `tool_name`, `tool_input`, `tool_response`, `tool_use_id`, optional `duration_ms` | tool completed |
| `PostToolUseFailure` | `tool_name` | `tool_name`, `tool_input`, `tool_response` or error details, `tool_use_id`, optional `duration_ms` | `error` |
| `PostToolBatch` | none | batch results | optional aggregate status |
| `Notification` | `notification_type` | `message`, optional `title`, `notification_type` | `waiting_approval` or attention |
| `SubagentStart` | agent type | `agent_id`, `agent_type` | optional subagent status |
| `SubagentStop` | agent type | `stop_hook_active`, `agent_id`, `agent_type`, `agent_transcript_path`, `last_assistant_message` | optional done status |
| `TaskCreated` | none | `task_id`, `task_subject`, optional `task_description`, `teammate_name`, `team_name` | optional team status |
| `TaskCompleted` | none | `task_id`, `task_subject`, optional `task_description`, `teammate_name`, `team_name` | optional team status |
| `Stop` | none | `stop_hook_active`, `last_assistant_message` | `done` |
| `StopFailure` | error type | `error`, optional `error_details`, optional `last_assistant_message` | `error` |
| `TeammateIdle` | none | `teammate_name`, `team_name` | optional attention |
| `InstructionsLoaded` | load reason | instruction load metadata | ignore initially |
| `ConfigChange` | source | `source`, optional `file_path` | ignore initially |
| `CwdChanged` | none | `old_cwd`, `new_cwd` | update cwd/session grouping |
| `FileChanged` | watched filenames | `file_path`, `event` with values such as `change`, `add`, or `unlink` | optional activity |
| `WorktreeCreate` | none | `name` | optional status |
| `WorktreeRemove` | none | `worktree_path` | optional status |
| `PreCompact` | trigger | `trigger`, `custom_instructions` | optional compacting status |
| `PostCompact` | trigger | `trigger`, `compact_summary` | optional done status |
| `SessionEnd` | reason | `reason` | session ended |
| `Elicitation` | MCP server | `mcp_server_name`, `message`, optional `mode`, `url`, `elicitation_id`, `requested_schema` | `waiting_approval` |
| `ElicitationResult` | MCP server | `mcp_server_name`, `action`, optional `mode`, `elicitation_id`, `content` | optional status |

### Claude Code Tool Inputs

For `PreToolUse`, `PermissionRequest`, `PostToolUse`, and
`PostToolUseFailure`, `tool_input` depends on the tool. The reference explicitly
documents common built-in shapes:

| Tool | Notable `tool_input` fields |
| --- | --- |
| `Bash` | `command`, optional `description`, optional `timeout` in milliseconds, optional `run_in_background` |
| `Write` | `file_path`, `content` |
| `Edit` | `file_path`, `old_string`, `new_string`, `replace_all` |
| `Read` | file-specific read arguments |
| `Glob` | glob pattern arguments |
| `Grep` | search pattern/path arguments |
| `Agent` | subagent/task arguments |
| `WebFetch` | URL and prompt arguments |
| `WebSearch` | query arguments |
| `AskUserQuestion` | question fields |
| `ExitPlanMode` | plan/exit fields |
| MCP tools | names follow `mcp__<server>__<tool>` and arguments are server-specific |

Agent Pets should only inspect a small safe subset:

- command preview: `tool_input.command`
- file path preview: `tool_input.file_path`
- notification text: `message`, `title`, `notification_type`
- error/failure text: `reason`, `error`, `error_details`

### Claude Code Output Rules Relevant to Agent Pets

Agent Pets should use side-effect-only hooks: no stdout and `exit 0`.

Important upstream behavior:

- `PreToolUse` can allow, deny, ask, defer, or modify tool input.
- `PermissionRequest` can allow or deny on behalf of the user and can update
  permissions.
- `PostToolUse` can add context or replace tool output visible to Claude.
- `Notification` hooks cannot block or modify notifications.
- Async command hooks are supported and should be preferred for Agent Pets where
  the event does not need control flow.

## GitHub Copilot

Source: https://docs.github.com/en/enterprise-cloud@latest/copilot/reference/hooks-reference

### Config and Runtime

- Hooks are supported in Copilot CLI and Copilot cloud agent.
- Copilot CLI hooks run locally in the same shell as the CLI.
- Hook sources are loaded in order and combined: repository `.github/hooks/*.json`,
  user `~/.copilot/hooks/*.json` or `$COPILOT_HOME/hooks/*.json`, inline
  repository settings, inline user settings, and installed plugins.
- Hook configuration files use JSON with `"version": 1`.
- Command hooks use `bash`, `powershell`, or `command`, plus optional `cwd`,
  `env`, and `timeoutSec`. Default `timeoutSec` is `30`.
- HTTP hooks POST the input payload as JSON and also have `timeoutSec`.
- Two payload formats exist:
  - camelCase format when configured with camelCase event keys, e.g.
    `preToolUse`
  - VS Code compatible format when configured with PascalCase event keys, e.g.
    `PreToolUse`
- Agent Pets should support both documented formats. PascalCase configuration
  provides `hook_event_name`; camelCase configuration omits it, so the adapter
  must infer the event from the documented payload shape.

### Copilot Events and Inputs

| Event key | VS Code compatible key | camelCase fields | VS Code compatible fields | Agent Pets use |
| --- | --- | --- | --- | --- |
| `sessionStart` | `SessionStart` | `sessionId`, `timestamp`, `cwd`, `source`, optional `initialPrompt` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `source`, optional `initial_prompt` | ready/resumed |
| `sessionEnd` | `SessionEnd` | `sessionId`, `timestamp`, `cwd`, `reason` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `reason` | session ended |
| `userPromptSubmitted` | `UserPromptSubmit` | `sessionId`, `timestamp`, `cwd`, `prompt` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `prompt` | `thinking` |
| `preToolUse` | `PreToolUse` | `sessionId`, `timestamp`, `cwd`, `toolName`, `toolArgs` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `tool_name`, `tool_input` | `running` or `editing` |
| `postToolUse` | `PostToolUse` | `sessionId`, `timestamp`, `cwd`, `toolName`, `toolArgs`, `toolResult` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `tool_name`, `tool_input`, `tool_result` | tool completed |
| `postToolUseFailure` | `PostToolUseFailure` | `sessionId`, `timestamp`, `cwd`, `toolName`, `toolArgs`, `error` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `tool_name`, `tool_input`, `error` | `error` |
| `agentStop` | `Stop` | `sessionId`, `timestamp`, `cwd`, `transcriptPath`, `stopReason` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `transcript_path`, `stop_reason` | `done` |
| `subagentStart` | none documented in VS Code table | `sessionId`, `timestamp`, `cwd`, `transcriptPath`, `agentName`, optional `agentDisplayName`, optional `agentDescription` | not documented in the captured table | optional subagent status |
| `subagentStop` | `SubagentStop` | `sessionId`, `timestamp`, `cwd`, `transcriptPath`, `agentName`, optional `agentDisplayName`, `stopReason` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `transcript_path`, `agent_name`, optional `agent_display_name`, `stop_reason` | optional subagent done |
| `errorOccurred` | `ErrorOccurred` | `sessionId`, `timestamp`, `cwd`, `error`, `errorContext`, `recoverable` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `error`, `error_context`, `recoverable` | `error` |
| `preCompact` | `PreCompact` | `sessionId`, `timestamp`, `cwd`, `transcriptPath`, `trigger`, `customInstructions` | `hook_event_name`, `session_id`, `timestamp`, `cwd`, `transcript_path`, `trigger`, `custom_instructions` | compacting |
| `notification` | `Notification` | `sessionId`, `timestamp`, `cwd`, `hook_event_name`, `message`, optional `title`, `notification_type` | same documented shape; this event already includes `hook_event_name` | attention |
| `permissionRequest` | `PermissionRequest` | permission request payload; matcher is tested against `toolName` | permission request payload with `hook_event_name` when PascalCase is supported | `waiting_approval` |

The Copilot reference notes that CLI supports all described events. Cloud agent
uses a subset and runs in an ephemeral sandbox, which is not relevant for local
Agent Pets user-level integration.

Copilot notification types documented for CLI:

- `shell_completed`
- `shell_detached_completed`
- `agent_completed`
- `agent_idle`
- `permission_prompt`
- `elicitation_dialog`

Copilot tool names documented for matcher filtering:

- `ask_user`
- `bash`
- `create`
- `edit`
- `glob`
- `grep`
- `powershell`
- `task`
- `view`
- `web_fetch`

### Copilot Decision and Exit Rules Relevant to Agent Pets

Agent Pets should be observational:

- Command hook exit `0` means success; stdout is parsed as hook output JSON if
  present.
- Exit `2` is treated as a warning by default, with special behavior for
  permission and failure hooks.
- Other non-zero exits are logged as hook failures but the run continues.
- `preToolUse` can return permission decisions and modified args. Agent Pets
  should not use that path.
- Because default command hook timeout is `30s`, generated Agent Pets config
  must set a tiny `timeoutSec`, ideally `1`, and the adapter must still have its
  own `100-250ms` internal timeout.

## Normalization Rules for `agent-pets hook <source>`

The adapter should parse these fields in order.

### Event Name

| Source | Primary field | Fallbacks |
| --- | --- | --- |
| Codex | `hook_event_name` | optional CLI event arg only for future compatibility |
| Claude Code | `hook_event_name` | optional CLI event arg only for future compatibility |
| Copilot | `hook_event_name` for PascalCase config | infer the event from documented camelCase payload shape |

### Session and Location

| Normalized field | Codex | Claude Code | Copilot PascalCase | Copilot camelCase |
| --- | --- | --- | --- | --- |
| `session_id` | `session_id` | `session_id` | `session_id` | `sessionId` |
| `cwd` | `cwd` | `cwd` | `cwd` | `cwd` |
| `timestamp` | adapter generated if missing | adapter generated if missing | `timestamp` | convert millisecond `timestamp` |
| `transcript_path` | `transcript_path` | `transcript_path` | `transcript_path` | `transcriptPath` |

### Tool Fields

| Normalized field | Codex | Claude Code | Copilot PascalCase | Copilot camelCase |
| --- | --- | --- | --- | --- |
| `tool_name` | `tool_name` | `tool_name` | `tool_name` | `toolName` |
| `tool_input` | `tool_input` | `tool_input` | `tool_input` | `toolArgs` |
| `tool_response` | `tool_response` | `tool_response` | `tool_result` | `toolResult` |
| `tool_use_id` | `tool_use_id` | `tool_use_id` | unavailable | unavailable |

### State Mapping

| Event family | State | Label preference |
| --- | --- | --- |
| session start/resume | `done` | `Ready` |
| user prompt submitted | `thinking` | `Thinking` |
| pre tool use, shell | `running` | `Running shell` |
| pre tool use, edit/write/apply_patch | `editing` | `Editing` |
| pre tool use, other tool | `running` | `Using tool` |
| permission request or elicitation | `waiting_approval` | `Waiting approval` |
| notification requiring attention | `waiting_approval` | `Needs attention` |
| post tool success | `running` | `Tool completed` |
| post tool failure or error occurred | `error` | `Tool failed` |
| stop/session end | `done` | `Done` |
