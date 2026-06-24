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

## Frontend without Tauri (web preview / Cloudflare)

The Vite frontend in `app/` runs standalone — no Rust build required — so the
look and feel can be verified in a plain browser or shared on Cloudflare.

- All Tauri touch points (`invoke` / `listen` / window dragging) are funnelled
  through `app/src/bridge.ts`. Inside the desktop app it forwards to
  `@tauri-apps/api`; in a plain browser it falls back to a self-contained mock,
  so the frontend never hard-depends on Tauri.
- `pnpm run build:web` produces a Tauri-free bundle in `app/dist/`, with the
  `playground` page emitted as `index.html`. `pnpm run preview:web` serves it.

### Deploy to Cloudflare (Workers Static Assets)

`app/wrangler.jsonc` serves `app/dist` with no Worker code. From the Cloudflare
dashboard (Workers Builds):

- **Root directory:** `app`
- **Build command:** `pnpm install && pnpm run build:web`
- **Deploy command:** `npx wrangler deploy` (production) /
  `npx wrangler versions upload` (preview branches)

No Tauri, Rust, or cargo is built for the web target. See
`docs/superpowers/specs/2026-06-24-navi-ui-redesign-design.md` for the full
design.

## Repository

Repository name stays `agent-pets` (the pet that hosts a NetNavi).
Derived from [Codex Pets](https://github.com/openai/codex).
