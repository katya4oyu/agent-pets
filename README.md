# Agent Pets

Desktop companions for agent progress, notifications, and attention requests.

Agent Pets is a small desktop resident app concept inspired by Codex pets. The
first milestone focuses on one pet, one speech bubble, and reliable status
updates from Codex Hooks. Claude Code and GitHub Copilot CLI can be added later
through the same local event API.

## First milestone

- Single floating pet overlay.
- Speech bubble with the latest notification and current state.
- Local event receiver for hook adapters.
- Codex Hooks adapter first.
- Tauri v2 app shell with tray controls.

## Target states

- `thinking`: user prompt submitted, agent is planning or reasoning.
- `running`: tool or command is running.
- `editing`: file edit is in progress.
- `waiting_approval`: agent needs permission or input.
- `done`: turn completed.
- `error`: tool or hook failure worth surfacing.

## Architecture

```text
Codex Hooks / Claude Code Hooks / Copilot CLI Hooks
        |
        v
curl http://127.0.0.1:<port>/events/<source>
        |
        v
Tauri app event store
        |
        v
Floating pet + speech bubble
```

## Recommended stack

- Tauri v2 for the desktop app, tray, transparent overlay, and Rust backend.
- A small Rust HTTP receiver bound to localhost.
- A web frontend for the pet overlay UI.
- Hook commands that pipe stdin directly to the local receiver with `curl`.
  The desktop app normalizes each agent payload server-side and exits the hook
  command quickly without blocking the agent.

## Repository status

This repository is currently a handoff scaffold. The next step is to create the
Tauri v2 app and implement the Codex hook adapter.
