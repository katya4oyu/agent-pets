# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

See @README.md for overview, @docs/concept.md for project concept and **important terminology** (read it — past sessions lost time confusing "codex 本家" with `codex-pet-web`).

## Commands

pnpm workspace; all frontend code lives under `app/`. Run `pnpm install` at the root first.

```sh
pnpm dev              # Vite dev server for the frontend (127.0.0.1:1420)
pnpm tauri:dev        # run the full Tauri desktop app (Rust + webview)
pnpm build            # frontend build for Tauri → app/dist (runs tsc, then vite build)
pnpm build:playground # Tauri-free build; emits playground.html as index.html → app/dist (Cloudflare)
pnpm preview:playground # serve the build:playground output
pnpm test             # vitest (frontend unit tests)
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
- **Frontend↔Tauri build coupling**: `app/src-tauri/tauri.conf.json` sets `frontendDist: ../dist` and `beforeBuildCommand: pnpm build`. Vite has two entries — `index.html` (Tauri shell → `main.ts`) and `playground.html` (browser sandbox → `playground.ts`). `--mode playground` (`build:playground`) rewrites the playground to `index.html`; the root `wrangler.jsonc` serves that output (`assets.directory: ./app/dist`) and is deployed from the repo root.
- **Sprite rendering follows the codex-compatible atlas** (8×9, 192×208; `docs/codex-pet-spritesheets.md`). `app/src/pet/` (`navi-pet.ts` web component + `pet-core.ts`) is the "codex-pet" responsibility; the navi-specific UI (bubbles/badges/buttons) is separate. Planned split into `packages/ui` is recorded in @docs/frontend-packaging.md (not yet implemented).

## Project rules / gotchas

- **Don't pre-implement the future vision.** The navi roadmap (Operator Core / Skills / Outbound) is aspirational; current safe scope is **Phase 1 only — non-breaking internal refactors** (`docs/navi-roadmap.md`). Confirm before structural changes.
- **Product is named "navi"** but the repo/config stay `agent-pets`; rename is deferred (`issues/3d107c`).
- **pnpm 11 build approval**: esbuild (a Vite dependency) needs a build-script approval, set via `allowBuilds: esbuild: true` in `pnpm-workspace.yaml`. This lets `pnpm install` / `pnpm run` work from the repo root (how Cloudflare runs `pnpm install && pnpm build:playground`). Don't remove it.
