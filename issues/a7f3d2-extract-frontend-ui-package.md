---
id: a7f3d2
title: フロントエンドを packages/ui に切り出す（codex-pet はその内部に分離）
type: chore
status: todo
priority: high
---

フロント表示層を `app`（Tauri）から切り出し、playground と app が同じ部品を使う構造にする。
背景・用語・決定の根拠は `docs/frontend-packaging.md` / `docs/concept.md` / `CLAUDE.md` を参照。

**未着手。実装はオーナーの明示的なゴー待ち。**

## ゴール

- `packages/ui`（唯一の公開パッケージ）を新設。
- `ui` 内部で **codex-pet（スプライト描画 = `navi-pet` + `pet-core`）をコードとして分離**（例 `packages/ui/src/codex-pet/`）。navi 固有 UI（吹き出し・バッジ・ボタン・セッションカウント）は別ディレクトリ。
- `examples/playground` を `ui` を import する独立アプリ化（デザイン微調整 + パラメータ露出のスライダー）。
- `app`（Tauri シェル）も `ui` を import。シェル配線（bridge / sessions / イベント / トレイ）は app に残す。
- Cloudflare 公開対象を `app` から `examples/playground` へ移設（`build:web` / `wrangler.jsonc` を playground へ）。

## 想定タスク（順序は要相談）

- [ ] pnpm workspace に `packages/*` / `examples/*` を追加。
- [ ] `packages/ui` 作成、`app/src/pet/`（navi-pet, pet-core）を `ui/src/codex-pet/` へ移設。
- [ ] navi 固有 UI（`main.ts` 内の吹き出し生成・バッジ・ボタン）を `ui` へ抽出。
- [ ] `app` を `ui` 依存に差し替え（`tsc` / `vite build` / `vitest` 緑を維持。`frontendDist=../dist` を壊さない）。
- [ ] `examples/playground` を独立アプリ化、`wrangler.jsonc` / `build:web` を移設。
- [ ] CLAUDE.md / docs/frontend-packaging.md の「未確定・要確認」を解消（パッケージ名・要素名・資産配置・着手順）。

## 注意

- 将来像（Operator Core / Skill / Outbound 等）は先回り実装しない（`docs/navi-roadmap.md`）。本タスクは表示層の構成変更のみ。
- 状態の真実は将来 World Model（バックエンド）へ集約予定。本タスクで `sessions` のバックエンド移管までは踏み込まない。
