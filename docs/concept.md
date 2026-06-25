# navi / agent-pets コンセプト

プロジェクトの背景・思想・用語をまとめた概念ドキュメント。
（運用ルール＝コマンド・規約は `CLAUDE.md`、深い設計は `docs/navi-architecture.md` 等を参照。）

## これは何か

- **現状**: `agent-pets` は、コーディングエージェント（**OpenAI Codex / Claude Code / GitHub Copilot CLI**）のライフサイクル hook を受け取り、デスクトップ常駐のペットとして状態を可視化する**受動的な通知ビューア**。
- **将来像「navi」**: ロックマンEXE のネットナビ的な、能動的オペレーター・コンパニオンへ段階的に育てる構想。詳細は `docs/navi-architecture.md` / `docs/navi-roadmap.md`。
- **実装ゲート（重要）**: 将来像（Operator Core / Skill / Outbound 操縦 等）は「体験設計＋E2E テスト計画が固まるまで先回り実装しない」。現状の安全圏は **Phase 1（外部仕様を変えない内部リファクタ）まで**。
- **リネーム据え置き**: GitHub リポジトリ名は `agent-pets` のまま。製品表示名は `navi`。

## 用語（取り違え注意）

過去に取り違えて手戻りした箇所。明確に区別すること。

> 用語の**厳密な対応表（コード識別子つき）は `docs/glossary.md` を正**とする。
> 本節は背景説明にとどめ、名前の確定は glossary を参照すること。

- **「codex 本家」= OpenAI Codex の pets 機能のスプライト／アトラス規約**（`~/.codex/pets/<id>/` の `pet.json` + `spritesheet.webp`、8×9・192×208・行ごとに idle/running/.../review が固定。`docs/codex-pet-spritesheets.md`）。**`codex-pet-web` のことではない。**
- **`codex-pet-web` = リポジトリオーナー自作の「練習リポジトリ」**（codex pet スプライトを Web Component で描くために作ったもの）。本家でも上流でもない。**必要コードは navi に直接移植済み**（`app/src/pet/navi-pet.ts` ← `<codex-pet>`、`app/src/pet/pet-core.ts` ← MoonBit `pet_core.mbt`。MoonBit は排除）。**外部依存・上流追従は不要。**
- **責務は「出自」で切る**:
  - **codex-pet（責務名）** = codex のスプライトの作りに依存する描画／アニメ層。
  - **ui** = navi / agent-pets 固有の表現要件（吹き出しスタック・ソースバッジ・トグル/リサイズ/設定ボタン・セッションカウント）。codex 由来ではない。

## アーキテクチャ概観

```text
Codex / Claude Code / Copilot CLI
   │  hook → stdin JSON
   ▼
navi-hook (Rust CLI)  →  POST /events/<source>   (fire-and-forget, ~100-250ms)
   ▼
Tauri backend (app/src-tauri)  … tiny_http 受信 / normalize() / emit "agent-state-changed"
   ▼
Frontend (app/src/main.ts)  … sessions Map・最優先 state でアニメ・吹き出しスタック描画
```

- 目標 6 層（Avatar / Skill / Operator Core / World Model / Event Bus / Connector）は `docs/navi-architecture.md`。フロントは最上層の **Avatar / Presentation 層**。
- 状態の真実は将来 **World Model としてバックエンドへ集約**予定（現状は `main.ts` 内の `sessions`）。

## フロントのパッケージ分割（実施済み・一部継続）

- パッケージは **`packages/ui`（`@navi/ui`）の 1 つだけ**。**内部で codex-pet をコードとして分離**（`packages/ui/src/codex-pet/` = `navi-pet` + `pet-core`）。
- **`examples/playground`** = `@navi/ui` を import する独立アプリ（Cloudflare 公開対象）。
- navi 固有 UI（ステータスカード・ソースバッジ・状態モデル）は `@navi/ui`（`src/navi/`）へ抽出済み（`c4b1e0` 完了）。app シェルと playground の両方が利用する。app は依然スプライトを独自 canvas で描画（`<navi-pet>` への移行は `e1f5c3` で別途）。
- 詳細・残課題は `docs/frontend-packaging.md`、TODO は `issues/a7f3d2-extract-frontend-ui-package.md`。
