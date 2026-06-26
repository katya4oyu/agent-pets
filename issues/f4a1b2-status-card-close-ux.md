---
id: f4a1b2
title: 右上ステータスアイコンの図像セット（state→アイコン）と accent の降格
type: design
status: in-progress
priority: high
---

> 更新: 「閉じる」操作は **D4 で決定済み**（左上の丸いアイコンボタン、ソースバッジと hover で morph）。
> playground に試作済み。本 issue の残りは **右上のステータスアイコン図像セット**（D3 の実装）。

> 前提（既決・`docs/decisions.md`）: D1 自動消滅させない（手動 dismiss 必須）／D2 app は desktop のみ
> （hover を一級の手段として使える）／D3 state の一次表現は右上のステータスアイコン。
> 原則: `docs/design-principles.md`（P2 静けさ / P3 形が先 / P4 光学）。

## 背景

本家 codex pets は右上の角にステータスアイコンを置き、閉じるは hover 前提（desktop 専用）。
我々も D3 で右上をステータスアイコンに使うと決めたため、**「閉じる」操作の置き場所が空く**。
D1 で手動 dismiss は必須なので、独自に設計する。

## 閉じる操作（決定済み・D4）

左上の丸いアイコンボタン。既定はソースバッジ、カード hover で crossfade して✖へ morph。
playground 実装済み（`examples/playground/src/playground.css` の `.source-badge` / `.status-card-close`）。
→ 詳細は `docs/decisions.md` D4 / `docs/status-card-design.md`。

## 残りの論点（未決）

右上の**ステータスアイコン図像セット**（D3 の具体化）。

## 決め方 / 検証

- playground に試作（右上ステータスアイコン＋ hover クローズ）。state→アイコン対応も同時に確認:
  `done`✓ / `running`◴ / `editing`✎ / `thinking`✦ / `waiting_approval`❗ / `error`⚠（アテンションのみ色＋微パルス）。
- 左レール accent は周辺視の補助として弱く残すか、ここで合わせて判断。
- 確定したら `docs/decisions.md` D3 の「未決」を更新（or 新 Dn）し、具体は `docs/status-card-design.md` へ。
