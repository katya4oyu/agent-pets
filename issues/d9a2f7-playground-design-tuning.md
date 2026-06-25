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

## 進捗

- [x] playground を pet × navi UI 統合レイアウトへ刷新（透過オーバーレイ再現の stage に
      吹き出しスタック・ソースバッジ・トグル/セッションカウントを重ねた）。
- [x] navi 固有 UI は playground ローカルに自己完結実装（`examples/playground/src/navi-ui.ts`）。
      `@navi/ui` への抽出（`c4b1e0`）は本 issue 範囲外のため未実施。ここで確定した構造/既定値を
      後で抽出側へ焼き込む前提。
- [x] パラメータ露出: ペットサイズ・アニメ fps、吹き出し（最大幅/余白/角丸/オフセット/影/尻尾）、
      スタック（gap・最大表示件数→以降スクロール）、ソース別カラー、表示モード（show/hide/auto）。
- [x] セッション操作（source/state/タイトル/メッセージ/プロジェクト編集・追加/削除）と
      オーバーフロー再現プリセット（Long message / Fill ×5 / Clear）。
- [x] 現在値を CSS カスタムプロパティ形式で表示しコピーできる readout を実装。
- [ ] オーナーが Cloudflare 上で確認し、値を詰めて確定（→ 抽出側 `c4b1e0` へ反映）。
