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
            "command": "agent-pets-hook codex SessionStart",
            "statusMessage": "Updating Agent Pets"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "agent-pets-hook codex UserPromptSubmit"
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
            "command": "agent-pets-hook codex PreToolUse"
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
            "command": "agent-pets-hook codex PermissionRequest"
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
            "command": "agent-pets-hook codex PostToolUse"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "agent-pets-hook codex Stop"
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
2. Normalize it to the Agent Pets event schema.
3. POST it to the local Agent Pets receiver.
4. Exit quickly.
5. Do not emit stdout by default.

Failures should not block Codex. If the app is not running, the adapter can
silently exit with status `0` after a short timeout.
