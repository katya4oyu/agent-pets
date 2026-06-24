---
id: a7f3d2
title: フロントエンドを packages/ui に切り出す（codex-pet はその内部に分離）
type: chore
status: done
priority: high
---

フロント表示層を `app`（Tauri）から切り出し、playground と app が同じ部品を使う構造にする。
背景・用語・決定の根拠は `docs/frontend-packaging.md` / `docs/concept.md` / `CLAUDE.md` を参照。

**この issue のスコープ（パッケージ分割 + playground 独立アプリ化）は完了。**
続きは独立 issue に分割した: `c4b1e0`（navi 固有 UI 抽出）/ `d9a2f7`（playground チューニング環境）/ `e1f5c3`（app シェル移行）。

## ゴール

- `packages/ui`（唯一の公開パッケージ）を新設。
- `ui` 内部で **codex-pet（スプライト描画 = `navi-pet` + `pet-core`）をコードとして分離**（例 `packages/ui/src/codex-pet/`）。navi 固有 UI（吹き出し・バッジ・ボタン・セッションカウント）は別ディレクトリ。
- `examples/playground` を `ui` を import する独立アプリ化（デザイン微調整 + パラメータ露出のスライダー）。
- `app`（Tauri シェル）も `ui` を import。シェル配線（bridge / sessions / イベント / トレイ）は app に残す。
- Cloudflare 公開対象を `app` から `examples/playground` へ移設（`build:web` / `wrangler.jsonc` を playground へ）。

## 進捗

- [x] pnpm workspace に `packages/*` / `examples/*` を追加。
- [x] `packages/ui`（`@navi/ui`）作成、`app/src/pet/`（navi-pet, pet-core）を `ui/src/codex-pet/` へ移設。
- [x] `examples/playground` を独立アプリ化（playground/gallery + mio 資産を移設、`@navi/ui` を import）。
- [x] Cloudflare 公開対象を `examples/playground/dist` へ（root `wrangler.jsonc` / `build:playground`）。
- [x] 既定の確定: パッケージ名 `@navi/ui`、要素名 `<navi-pet>` 維持、mio は playground にコピー。
- [x] 完了（コミット `dbc19e6` / `160506b`）。続きは下記 issue へ。

## 続き（別 issue）

- `c4b1e0` — navi 固有 UI（吹き出し・バッジ・ボタン・セッションカウント）を `@navi/ui` へ抽出。
- `d9a2f7` — playground を pet×UI 統合デザインのチューニング環境にする（パラメータをスライダーで露出）。
- `e1f5c3` — app シェル（`main.ts`）を codex-pet（`<navi-pet>`）描画へ移行。

## 注意

- 将来像（Operator Core / Skill / Outbound 等）は先回り実装しない（`docs/navi-roadmap.md`）。本タスクは表示層の構成変更のみ。
- 状態の真実は将来 World Model（バックエンド）へ集約予定。本タスクで `sessions` のバックエンド移管までは踏み込まない。
