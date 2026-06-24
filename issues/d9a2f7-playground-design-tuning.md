---
id: d9a2f7
title: playground を pet×UI 統合デザインのチューニング環境にする（パラメータ露出）
type: feature
status: todo
priority: high
---

> 前提を読むこと: `CLAUDE.md`、`docs/concept.md`、`docs/frontend-packaging.md`、
> `docs/superpowers/specs/2026-06-24-navi-ui-redesign-design.md`（§4 開発環境 / §6 見た目の作り込み）。
> 望ましくは `c4b1e0`（navi UI 抽出）の後。少なくとも codex-pet は `@navi/ui` から利用可能。

## 意図（オーナーのワークフロー）

playground は **pet アバター × navi UI の統合デザインを微調整する場**。スライダー等でパラメータを露出し、
**オーナーが良い値を読み取って指示 → エージェントが実装（コンポーネント既定値 / シェル）へ焼き込む**。
playground は「動くシェルを量産する場」ではなく、**パラメータと design を確定させるサンドボックス**。

## 現状

- `examples/playground`（独立アプリ・Cloudflare 公開対象）は `<navi-pet>` を表示し、state ボタンとサイズスライダーのみ（`src/playground.ts`）。吹き出し等の navi UI は未表示。

## ゴール

- playground 上で **pet と navi UI を統合したレイアウト**を構成（吹き出しスタック・バッジ・ボタン・セッションカウント）。
- 調整したいパラメータをコントロール（スライダー/入力/トグル）で露出。例:
  - ペットサイズ、吹き出しの位置・尻尾・余白・最大幅・角丸・影・配色
  - 複数セッション時の積み重ね方・スクロール挙動（`issues/b3f2a1`）
  - アニメ timing、ソース別の色
  - 長文メッセージ / 長いプロジェクト名のオーバーフロー再現
- 現在の値を画面に表示し、オーナーが書き出せるようにする（コピーしやすい形）。

## 範囲外

- 抽出そのもの（`c4b1e0`）と app シェル移行（`e1f5c3`）。

## 検証

- `pnpm build:playground` 緑、Cloudflare（`examples/playground/dist`）で確認できる。
- オーナーがスライダーで詰めた値を一覧で読み取れる。
