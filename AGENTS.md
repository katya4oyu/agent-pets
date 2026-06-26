# AGENTS.md

あらゆるエージェント（Claude Code / Codex / Copilot 等）共通の作業規約。
**詳細の正は `CLAUDE.md`**。本書はツールを問わず守る最小規律だけを置く。

## 最優先：決定ログ

- 重要な決定は **`docs/decisions.md`（追記専用）** に集約する。
- **蒸し返す前に必ず読む。** そこに反する提案・実装をしない。
- 会話で重要な決定をしたら、**実装と同じコミットでその場で 1 項目追記**する。口頭で決めて記録しない運用は禁止。
- 既決（覆すには decisions.md で明示的に Supersede）:
  - **D1** ステータスカードは自動消滅させない（手動 dismiss まで残す。通知見逃し防止）。
  - **D2** app は desktop のみ対象。モバイル考慮は playground 限定。
  - **D3** state の一次表現は右上のステータスアイコン（色は補助・アテンションのみ）。

## ドキュメントの用途分担（迷ったらここ）

| 種類 | 置き場所 |
|---|---|
| 決定（理由・却下案つき） | `docs/decisions.md` |
| 用語の正（コード識別子つき） | `docs/glossary.md` |
| 思想・背景 | `docs/concept.md` |
| 視覚デザインの思想/原則 | `docs/design-principles.md` |
| 視覚デザインの具体値 | `docs/status-card-design.md` |
| アーキ設計 / ロードマップ | `docs/navi-architecture.md` / `docs/navi-roadmap.md` |
| 未決の論点・TODO | `issues/` |
| 開発コマンド・落とし穴 | `CLAUDE.md` |

## 基本動作

- 作業前に `CLAUDE.md` と関連 `docs/` を読む。命名は `glossary.md` に従う。
- 構造変更は Phase 1（外部仕様を変えない内部リファクタ）まで。先回り実装はしない（`navi-roadmap.md`）。
- 決定・設計に変更が出たら、コードと**同じコミットで**該当ドキュメントを更新する（記録の後回し禁止）。
