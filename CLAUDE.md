# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

See @README.md for overview, @docs/concept.md for project concept, and @docs/glossary.md for the **strict terminology table** (authoritative names + code identifiers — read it before naming UI parts; e.g. the cards above the avatar are **ステータスカード / status card**, not 吹き出し/toast/speech. Past sessions lost time confusing "codex 本家" with `codex-pet-web`). For the **visual design intent** of the status card (source badge variants / optical sizing / cap-height alignment, and the shadow model — gap-contained negative spread, positional/raking light), see @docs/status-card-design.md.

## Layout

pnpm workspace (`app`, `packages/*`, `examples/*`). Run `pnpm install` at the root first.

- `app/` — the Tauri desktop app (frontend `app/src` + Rust `app/src-tauri`).
- `packages/ui` (`@navi/ui`) — navi presentation layer. `src/codex-pet/` is the codex-compatible sprite renderer; `src/navi/` is the navi-specific UI (status card / source badge / state model), consumed by both the app shell and the playground.
- `examples/playground` — standalone browser sandbox (imports `@navi/ui`) for tuning the pet + UI design. Cloudflare deploy target.

## Commands

```sh
pnpm dev              # Vite dev server for the Tauri frontend (127.0.0.1:1420)
pnpm tauri:dev        # run the full Tauri desktop app (Rust + webview)
pnpm build            # Tauri frontend build → app/dist (runs tsc, then vite build)
pnpm build:playground # build examples/playground → its dist (Cloudflare; index.html = playground)
pnpm preview:playground # serve the playground build
pnpm test             # vitest (app unit tests)
pnpm check:rust       # cargo check on the Tauri crate
```

- Single / filtered test: `pnpm --dir app exec vitest run src/state.test.ts` or `... vitest run -t "name"`.
- Type-check only: `pnpm --dir app exec tsc`.
- Rust tests live in `app/src-tauri/src/lib.rs`: `cargo test --manifest-path app/src-tauri/Cargo.toml`.

## Architecture (big picture)

Event flow (one direction today — agent → pet):

```
navi-hook (Rust CLI, app/src-tauri/src/bin/navi_hook.rs)
  → POST /events/<source>  (fire-and-forget, ~100-250ms)
  → Tauri backend (app/src-tauri/src/lib.rs): tiny_http server + normalize() + emit "agent-state-changed"
  → frontend (app/src/main.ts): sessions Map, highest-priority state drives the sprite, one speech bubble per session
```

- **Pure logic is deliberately split out** for headless testing and a future backend World Model: `app/src/state.ts` (priority/labels/animation table, fully DOM/Tauri-free, covered by `state.test.ts`) and the Rust `agent-pets-core` crate (`app/src-tauri/core`, normalization/schema).
- **`app/src/bridge.ts` is the only Tauri touch point** in the frontend (wraps `invoke`/`listen`/window drag). Outside Tauri it falls back to a browser mock, so the frontend can run without the desktop app.
- **Frontend↔Tauri build coupling**: `app/src-tauri/tauri.conf.json` sets `frontendDist: ../dist` and `beforeBuildCommand: pnpm build`; `app` is a single Vite entry (`index.html` → `main.ts`). The browser sandbox is a separate app (`examples/playground`, its own `index.html`); the root `wrangler.jsonc` serves `./examples/playground/dist` and deploys from the repo root.
- **Sprite rendering follows the codex-compatible atlas** (8×9, 192×208; `docs/codex-pet-spritesheets.md`). The `<navi-pet>` web component + `pet-core` live in `packages/ui/src/codex-pet/` (used by the playground). Note `app/src/main.ts` does **not** use `<navi-pet>` yet — the Tauri shell still renders sprites on its own `<canvas>` (migration pending: `issues/e1f5c3`). The navi-specific UI (status card / source badge / state model) **is** extracted into `@navi/ui` (`src/navi/`, issue `c4b1e0` done) and consumed by both the app shell and the playground.

## Project rules / gotchas

- **Don't pre-implement the future vision.** The navi roadmap (Operator Core / Skills / Outbound) is aspirational; current safe scope is **Phase 1 only — non-breaking internal refactors** (`docs/navi-roadmap.md`). Confirm before structural changes.
- **Product is named "navi"** but the repo/config stay `agent-pets`; rename is deferred (`issues/3d107c`).
- **Vite 8 / Node floor**: builds use Vite 8 (Vitest 4), which bundles with **Rolldown, not esbuild** (Rolldown forbids mutating the `generateBundle` `bundle` object — rename output files in `writeBundle` instead). Vite 8 needs Node `>=22.12` (or `>=20.19`).
