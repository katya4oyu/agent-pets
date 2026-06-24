---
id: c4b1e0
title: navi 固有 UI を @navi/ui に抽出する（吹き出し・バッジ・ボタン）
type: feature
status: todo
priority: high
---

> 前提を読むこと: `CLAUDE.md`、`docs/concept.md`（用語）、`docs/frontend-packaging.md`。
> 先行 issue `a7f3d2` でパッケージ分割は完了済み。

## 現状

- `packages/ui`（`@navi/ui`）は内部に **codex-pet**（`src/codex-pet/` = `<navi-pet>` + `pet-core`、スプライト描画）だけを持つ。
- navi 固有の UI（吹き出しスタック・ソースバッジ・トグル/リサイズ/設定ボタン・セッションカウント）は **`app/src/main.ts` にインラインで埋まっている**（`createBubbleElement` / `updateBubbleElement` / `sourceConfig` 等）。`app/src/styles.css` も同様。

## ゴール

navi 固有 UI を `@navi/ui`（codex-pet とは別ディレクトリ、例 `packages/ui/src/`）へ **再利用可能な「ダム」コンポーネント**として抽出する。app シェルと playground の両方が同じ部品を使えるようにする。

- 対象: 吹き出し（state / message / cwd / ソースバッジ）、ソースバッジ（claude-code / codex / copilot の `@lobehub/icons-static-svg`）、操作ボタン（トグル/リサイズ/設定）、セッションカウント。
- 部品は **props/属性 in・イベント out** に徹し、`sessions` 管理・`bridge`・イベント購読・トレイ連動などの**シェル配線は `app` 側に残す**。
- `@lobehub/icons-static-svg` 依存は `@navi/ui` 側へ移す（バッジが使うため）。

## 範囲外

- app シェルを `<navi-pet>` 描画へ移す件は別 issue `e1f5c3`。
- スライダーでのパラメータ露出は別 issue `d9a2f7`。
- World Model 等のバックエンド集約（`docs/navi-roadmap.md` Phase 1+）は触らない。

## 注意 / 仕様

- 吹き出しスタックは **最大3件、超過はスクロール**（`issues/b3f2a1`）。この挙動を保持。
- 透過オーバーレイ前提（ウィンドウ面積＝クリック不能域）なので闇雲に広げない。
- デザインの細部（尻尾・余白・配色・timing）は `d9a2f7` の playground ループで詰める想定。まずは構造を移すことを優先。

## 検証

- `app` の見た目・通知挙動が現状と同等（`pnpm --dir app build` / `pnpm --dir app test` 緑）。
- `examples/playground` から `@navi/ui` の navi UI を import して描画できる。
