# CLAUDE.md

このリポジトリで作業するエージェント（Claude / その他セッション）向けの入口。
**着手前に必ず読むこと。** 過去に背景理解の不足から手戻りが発生した。同じ過ちを繰り返さないための前提と作業規範をここにまとめる。

---

## 0. 作業規範（最重要・先に読む）

1. **調査を省かない。** 下の「必読ドキュメント」を読む前に提案・実装しない。憶測で構造や用語を決めない。
2. **勝手に解釈しない。** 分からない・確信が持てない点は放置せず、解釈で埋めずに**ユーザーに確認する**。
3. **ゴーが出てから実装する。** 設計・パッケージ構成・ディレクトリ再編などの判断を独断で進めない。ユーザーの**明示的な指示（ゴー）**を待つ。提案は良いが、合意前に書き換え・移動・ビルド構成変更をしない。
4. **将来像を先回り実装しない。** navi の野心的な構想（Operator Core / Skill / Outbound 操縦 等）は「体験設計＋E2E テスト計画が固まるまで着手しない」と設計書で明言されている（§原則7）。当面の安全圏は **Phase 1（外部仕様を変えない内部リファクタ）まで**。

---

## 1. プロジェクトの本質

- **現状**: `agent-pets` は、コーディングエージェント（**OpenAI Codex / Claude Code / GitHub Copilot CLI**）のライフサイクル hook を受け取り、デスクトップ常駐のペットとして状態を可視化する**受動的な通知ビューア**。
- **将来像**: ロックマンEXE のネットナビ的な、能動的オペレーター・コンパニオン「**navi**」へ段階的に育てる。詳細は `docs/navi-architecture.md` / `docs/navi-roadmap.md`。
- **リネーム据え置き**: GitHub リポジトリ名は `agent-pets` のまま。製品表示名は `navi`（`tauri.conf.json` productName 等）。「PET の中にネットナビが宿る」世界観（`issues/3d107c-rename-to-navi.md`）。

---

## 2. 必読ドキュメント（権威順）

| ドキュメント | 内容 |
| --- | --- |
| `docs/navi-architecture.md` | 目標 6 層アーキテクチャ（Avatar / Skill / Operator Core / World Model / Event Bus / Connector）。設計原則。 |
| `docs/navi-roadmap.md` | Phase 0–6 と実装ゲート。**当面は Phase 1 まで**。 |
| `docs/codex-pet-spritesheets.md` | **codex 互換スプライトのアトラス規約**（8×9・192×208・行ごとの状態）。 |
| `docs/superpowers/specs/2026-06-24-navi-ui-redesign-design.md` | フロント刷新／playground／Cloudflare の設計。 |
| `docs/frontend-packaging.md` | **フロントのパッケージ分割の決定**（本リポジトリでの合意事項）。 |
| `docs/handoff.md` / `docs/hook-*.md` | hook 連携の現行仕様・スキーマ。 |

---

## 3. 用語の取り違え注意（過去に間違えた箇所）

- **「codex 本家」= OpenAI Codex の pets 機能のスプライト／アトラス規約**を指す（`~/.codex/pets/<id>/` の `pet.json` + `spritesheet.webp`、8×9 グリッド・192×208・行ごとに idle/running/waving/.../review が固定。`docs/codex-pet-spritesheets.md`）。**`codex-pet-web` のことではない。**
- **`codex-pet-web` = リポジトリオーナーが自分で書いた「練習リポジトリ」**（codex pet スプライトを Web Component で描画するために作ったもの）。本家でも上流でもない。
  - **必要なコードは navi に直接移植済み**: `app/src/pet/navi-pet.ts`（`<codex-pet>` Web Component の移植）、`app/src/pet/pet-core.ts`（MoonBit `pet_core.mbt` の移植・MoonBit は排除）。**外部依存や上流追従は不要**。移植済みコードを扱うだけ。
- **責務は「出自」で切る**:
  - **codex-pet（責務名）** = codex のスプライトの作りに依存する描画／アニメ層（`navi-pet` + `pet-core`）。
  - **ui** = navi / agent-pets 固有の表現要件（吹き出しスタック・ソースバッジ・トグル/リサイズ/設定ボタン・セッションカウント）。codex 由来ではない。

---

## 4. フロント構成の決定（本セッションで合意）

> 詳細・根拠は `docs/frontend-packaging.md`。**まだ未実行（実装はゴー待ち）。**

- **パッケージは `packages/ui` の 1 つだけ**。理想は codex-pet と ui の並置だが、面倒なので分けない。
- **`ui` の内部で codex-pet を「コードとして」分離**（例: `packages/ui/src/codex-pet/`）。
- **`examples/playground`** = `ui` を import する独立アプリ。pet アバター × UI の**統合デザインを微調整**する場。スライダー等でパラメータを露出 → **オーナーが良い値を読み取り指示 → エージェントが実装へ焼き込む**。playground は動くシェルを量産する場ではない。
- **`app`（Tauri シェル）** も `ui` を import。**Cloudflare 公開対象は `examples/playground`**。

---

## 5. 現状コードの地図（要点）

- `app/` … Tauri アプリ。フロント = `app/src`、Rust = `app/src-tauri`。`tauri.conf.json` の `frontendDist: ../dist`、`beforeBuildCommand: pnpm build`。
- `app/src/main.ts` … navi シェル。`sessions` Map で全セッション状態を保持、最優先 state でアニメ切替、吹き出しスタックを描画。
- `app/src/bridge.ts` … **Tauri 抽象**（`invoke` / `listen` / ウィンドウドラッグ）。Tauri 不在のブラウザでは自己完結のモックにフォールバック。
- `app/src/pet/` … codex-pet 相当（`navi-pet.ts` Web Component + `pet-core.ts` アニメ tick）。
- `app/src/state.ts` … 純粋ロジック（状態優先度・ラベル・アニメ表）。DOM/Tauri 非依存。テスト = `app/src/state.test.ts`（vitest）。
- `app/src-tauri/core/` … `agent-pets-core` crate（正規化・スキーマ等の純 Rust ロジック）。
- 将来、状態の真実は **World Model としてバックエンドへ集約**予定（現状は `main.ts` 内）。

---

## 6. ビルド / テスト

```sh
pnpm install                 # ルートで（pnpm workspace, packages: ['app']）
pnpm --dir app build         # Tauri フロント → app/dist（tsc && vite build）
pnpm --dir app test          # vitest
pnpm --dir app build:web     # Tauri なしの web ビルド（playground を index.html 出力）
```

- Web プレビュー／Cloudflare 用に `app/wrangler.jsonc` と `build:web` が暫定で **`app/` 配下**にある。フロント分割後は **`examples/playground` 側へ寄せ直す**予定（`docs/frontend-packaging.md`）。
