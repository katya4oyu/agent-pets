# Codex Hooks Adapter Plan

## Sample hooks.json

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|resume",
        "hooks": [
          {
            "type": "command",
            "command": "'/Users/example/.agent-pets/bin/agent-pets' hook codex",
            "statusMessage": "Updating Agent Pets",
            "timeout": 1
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "'/Users/example/.agent-pets/bin/agent-pets' hook codex",
            "timeout": 1
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "'/Users/example/.agent-pets/bin/agent-pets' hook codex",
            "timeout": 1
          }
        ]
      }
    ],
    "PermissionRequest": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "'/Users/example/.agent-pets/bin/agent-pets' hook codex",
            "timeout": 1
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "'/Users/example/.agent-pets/bin/agent-pets' hook codex",
            "timeout": 1
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "'/Users/example/.agent-pets/bin/agent-pets' hook codex",
            "timeout": 1
          }
        ]
      }
    ]
  }
}
```

## Mapping

| Codex event | Agent Pets state | Bubble label |
| --- | --- | --- |
| `SessionStart` | `done` | `Ready` |
| `UserPromptSubmit` | `thinking` | `Thinking` |
| `PreToolUse` with `Bash` | `running` | `Running Bash` |
| `PreToolUse` with `apply_patch` | `editing` | `Editing` |
| `PreToolUse` with MCP tool | `running` | `Using tool` |
| `PermissionRequest` | `waiting_approval` | `Waiting approval` |
| `PostToolUse` success | `running` | `Tool completed` |
| `PostToolUse` failure | `error` | `Tool failed` |
| `Stop` | `done` | `Done` |

## Adapter behavior

1. Read JSON from stdin.
2. Read the Agent Pets port file.
3. POST the raw JSON to the local Agent Pets receiver.
4. Let the desktop app read `hook_event_name` and normalize the event.
5. Exit quickly with status `0`.
6. Do not emit stdout by default.

Failures should not block Codex. If the app is not running, the adapter can
silently exit with status `0` after a short timeout.

Keep the HTTP timeout small, around `100-250ms`, even though the Codex hook
configuration timeout is expressed in whole seconds. The configured `timeout: 1`
is only an outer guard; the adapter itself should usually return much faster.
