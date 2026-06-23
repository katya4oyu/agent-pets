# navi

Desktop companions for agent progress, notifications, and attention requests.

navi is a desktop resident app inspired by [Codex Pets](https://github.com/openai/codex). The
concept — visualizing coding agent state as a desktop pet — is extended to
support Claude Code, GitHub Copilot, and other agents through a common local
event API.

The name comes from the NetNavis of Mega Man Battle Network.

## Architecture

```text
Claude Code Hooks / Codex Hooks / Copilot CLI Hooks
        |
        v
navi-hook (CLI) → HTTP POST → localhost:<port>/events/<source>
        |
        v
Tauri app event store
        |
        v
Floating pet + speech bubble
```

## Agent states

- `thinking`: user prompt submitted, agent is planning or reasoning.
- `running`: tool or command is running.
- `editing`: file edit is in progress.
- `waiting_approval`: agent needs permission or input.
- `done`: turn completed.
- `error`: tool or hook failure worth surfacing.

## Stack

- Tauri v2 — desktop app, tray, transparent overlay, Rust backend.
- `tiny_http` — localhost HTTP receiver.
- Vite + TypeScript frontend — pet overlay UI.
- `navi-hook` CLI — installed to `~/.navi/bin/navi-hook`, piped from agent hooks.

## Repository

Repository name stays `agent-pets` (the pet that hosts a NetNavi).
Derived from [Codex Pets](https://github.com/openai/codex).
