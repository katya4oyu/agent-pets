---
id: e1f5c3
title: app シェル（main.ts）を codex-pet（<navi-pet>）描画へ移行する
type: refactor
status: todo
priority: medium
---

> 前提を読むこと: `CLAUDE.md`、`docs/concept.md`、
> `docs/superpowers/specs/2026-06-24-navi-ui-redesign-design.md`（フェーズ2 相当）。

## 現状（重要）

- **`app/src/main.ts`（Tauri シェル）は `<navi-pet>` を使っておらず、独自の `<canvas>` + `setTimeout` でスプライトを描画**している（`drawFrame` / `setAnimation`、アニメ表は `app/src/state.ts`）。
- 一方 `examples/playground` は `@navi/ui` の `<navi-pet>`（CSS background-position + rAF）で描画。
- つまり **描画系が二重**で、playground で詰めた見た目が app に自動反映されない。

## ゴール

- app シェルの canvas 描画を `@navi/ui` の `<navi-pet>` 利用へ置き換え、**playground と app が同一コンポーネントで描画**されるようにする（単一の真実）。
- これに伴い `app` は `@navi/ui` に依存（`workspace:*`）。
- `state.ts` の AgentState → アニメ対応は維持（`done=idle` / `running=running` / `waiting_approval=waving` / `error=failed` / `thinking=review`、`editing` は暫定 review）。

## 注意 / パリティ確認

- ドラッグ移動・右下リサイズ・サイズ範囲・`prefers-reduced-motion`・最優先 state でのアニメ切替が現状と同等であること。
- スプライト読込: app は Tauri 経由（`load_pet_asset` → `~/.codex/pets/<id>/`、`app/src/bridge.ts`）。`<navi-pet>` は `pet` 属性で manifest URL を fetch する設計なので、**Tauri 環境での資産供給方法を設計し直す必要がある**（bytes→ObjectURL を `<navi-pet>` に渡す等）。ここが最大の論点。
- 吹き出し等 navi UI は `c4b1e0` の成果物を利用。

## 範囲外

- 将来像（Operator Core / Skill / Outbound、World Model 集約）は先回りしない（`docs/navi-roadmap.md` は Phase 1 まで）。

## 検証

- `pnpm --dir app build` / `pnpm --dir app test` / `pnpm tauri:dev` で見た目・挙動が現状と同等。
