# フロントエンドのパッケージ分割（決定記録）

- 日付: 2026-06-24
- ステータス: **合意済み・未実装（実装はオーナーのゴー待ち）**
- 位置づけ: `docs/superpowers/specs/2026-06-24-navi-ui-redesign-design.md`（フロント刷新・playground・Cloudflare）の続き。同 spec は変更を `app/` 内に留めたが、本書は**フロントの再利用部分をパッケージ／独立アプリへ物理分割**する方針を確定する。

## 背景・動機

- フロント表示層（codex のスプライト描画 + navi 固有 UI）を、Tauri を起動せずに単体で見た目確認・デザイン調整できるようにしたい。さらに Cloudflare で共有したい。
- 再利用できる部分（スプライト描画、吹き出し等の UI）を `app`（Tauri）から切り出し、**playground（独立アプリ）と app の両方が同じ部品を使う**構造にする。

## 用語（重要・取り違え注意）

- **codex-pet（責務名）** = **OpenAI Codex の pets 機能のスプライト／アトラス規約**（`~/.codex/pets/<id>/`、8×9・192×208、行ごとに idle/running/.../review）に依存する**描画・アニメ層**。コードは `app/src/pet/`（`navi-pet.ts` + `pet-core.ts`）。
- **codex-pet-web** = リポジトリオーナーが書いた**練習リポジトリ**（本家でも上流でもない）。必要コードは navi に**直接移植済み**で、外部依存・追従は不要。
- **ui** = navi / agent-pets **固有**の表現要件：吹き出しスタック（マルチセッション、最大 3 件で以降スクロール = `issues/b3f2a1`）、ソースバッジ（claude-code / codex / copilot）、トグル/リサイズ/設定ボタン、セッションカウント。codex 由来ではない。

## 決定

1. **公開パッケージは `packages/ui` の 1 つのみ。**
   - 理想は codex-pet と ui を別パッケージとして並置することだが、運用が面倒なので**分けない**。
2. **`ui` の内部で codex-pet を「コードとして」分離する。**
   - 例: `packages/ui/src/codex-pet/`（`navi-pet` + `pet-core`）と、それ以外の navi 固有 UI を別ディレクトリに。
   - パッケージは 1 つでも、出自（codex 依存 / navi 固有）でコードの境界は保つ。
3. **`examples/playground` を独立アプリ化**（`ui` を import）。
   - 目的: pet アバター × UI の**統合デザインの微調整**。
   - 方式: スライダー等でパラメータ（サイズ・オフセット・吹き出し位置/尻尾/余白・配色・アニメ timing 等）を露出 → **オーナーが良い値を読み取って指示 → エージェントが実装へ焼き込む**。
   - playground は「動くシェルを量産する場」ではなく、**パラメータと design を確定させるサンドボックス**。
4. **`app`（Tauri シェル）も `ui` を import。**
   - シェル固有の配線（`bridge`、`sessions` 管理、イベント購読、トレイ連動）は app 側に残す（playground とは本質的に別物なので共通化しない）。
5. **Cloudflare 公開対象を `app` から `examples/playground` へ移す。**
   - 現在 `app/` にある暫定の `build:web` / `wrangler.jsonc` は playground 側へ寄せ直す。

## 想定構造（到達イメージ）

```text
agent-pets/
├── packages/
│   └── ui/                  … 唯一の公開パッケージ
│       └── src/
│           ├── codex-pet/   … スプライト描画（navi-pet + pet-core, codex 依存）
│           └── …            … navi 固有 UI（吹き出し・バッジ・ボタン 等）
├── examples/
│   └── playground/          … ui を import する独立アプリ（デザイン微調整・Cloudflare 公開対象）
└── app/                     … Tauri デスクトップ（ui を import、シェル配線は自前）
    ├── src/                 … main.ts(シェル) / bridge.ts / state.ts …
    └── src-tauri/           … Rust（core crate 等）
```

## 依存方向

- 理想は codex-pet と ui の**並置**だが、実体は **codex-pet コードが `ui` の内部モジュール**。
- `examples/playground` と `app` はともに `packages/ui` に依存。逆向き依存は作らない。

## Rust 側（必要に応じて）

- 純ロジックは既に `app/src-tauri/core`（`agent-pets-core` crate）へ切り出し済み。
- crate 化は**必要になった分だけ**。HTTP サーバ / hook CLI / トレイは Tauri 密結合なので投機的に crate 化しない。

## 未確定・要確認（実装前にオーナーに確認すること）

- パッケージ名（`@…/ui` のスコープ等）。
- カスタム要素名は現状 `<navi-pet>`。維持か変更か。
- スプライト資産（`mio` 等）の配置（`packages/ui` 同梱か、各アプリの public か）。
- 着手順（`packages/ui` 切り出しが先か、`examples/playground` 足場が先か）。

## 関連

- `CLAUDE.md` — 運用ルール（コマンド・規約・落とし穴）。
- `docs/concept.md` — プロジェクト概念・用語の取り違え注意。
- `docs/superpowers/specs/2026-06-24-navi-ui-redesign-design.md` — 直前のフロント刷新設計。
- `docs/navi-architecture.md` §10 Avatar 層 / §12 ディレクトリ。
- `issues/`（本決定の実行 TODO）。
