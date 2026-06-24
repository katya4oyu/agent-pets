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

## Workspace layout

pnpm workspace (`app`, `packages/*`, `examples/*`):

- `app/` — the Tauri desktop app (frontend `app/src` + Rust `app/src-tauri`).
- `packages/ui` (`@navi/ui`) — navi presentation layer. `src/codex-pet/` is the
  codex-compatible sprite renderer (`<navi-pet>` web component + `pet-core`).
- `examples/playground` — a standalone browser app (no Tauri) for tuning the
  pet + UI design; the Cloudflare deploy target.

## Design preview without Tauri (Cloudflare)

`examples/playground` runs the pet UI in a plain browser — no Rust build — so the
look and feel can be verified or shared without the desktop app.

- `pnpm run build:playground` builds `examples/playground` to its `dist/`
  (`index.html` = playground). `pnpm run preview:playground` serves it.
- The Tauri app's own Tauri touch points (`invoke` / `listen` / window dragging)
  are funnelled through `app/src/bridge.ts`, which falls back to a browser mock
  outside Tauri.

### Deploy to Cloudflare (Workers Static Assets)

The repo-root `wrangler.jsonc` serves `./examples/playground/dist` with no Worker
code. From the Cloudflare dashboard (Workers Builds):

- **Root directory:** repository root
- **Build command:** `pnpm install && pnpm run build:playground`
- **Deploy command:** `npx wrangler deploy` (production) /
  `npx wrangler versions upload` (preview branches)

No Tauri, Rust, or cargo is built for the web target. See `docs/concept.md` and
`docs/frontend-packaging.md` for the package split.

## Repository

Repository name stays `agent-pets` (the pet that hosts a NetNavi).
Derived from [Codex Pets](https://github.com/openai/codex).
