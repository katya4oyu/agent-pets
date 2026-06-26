---
id: f4a1b2
title: ステータスカードの「閉じる」操作を独自設計する（右上はステータスアイコン常設）
type: design
status: todo
priority: high
---

> 前提（既決・`docs/decisions.md`）: D1 自動消滅させない（手動 dismiss 必須）／D2 app は desktop のみ
> （hover を一級の手段として使える）／D3 state の一次表現は右上のステータスアイコン。
> 原則: `docs/design-principles.md`（P2 静けさ / P3 形が先 / P4 光学）。

## 背景

本家 codex pets は右上の角にステータスアイコンを置き、閉じるは hover 前提（desktop 専用）。
我々も D3 で右上をステータスアイコンに使うと決めたため、**「閉じる」操作の置き場所が空く**。
D1 で手動 dismiss は必須なので、独自に設計する。

## 論点（未決）

右上ステータス常設を保ったまま、desktop hover でどう閉じるか。

- **案1**: hover で✖が「ステータスアイコンの左隣」に出る（ステータスは少し左へ寄る）。
  状態を一切隠さない（P4）／常設ボタン無し（P2）。動きがやや増える。
- **案2**: ステータスアイコンが hover で✖に変化（同じ場所で二役）。要素最小だが hover 中は状態が隠れる。

## 決め方 / 検証

- playground に試作（右上ステータスアイコン＋ hover クローズ）。state→アイコン対応も同時に確認:
  `done`✓ / `running`◴ / `editing`✎ / `thinking`✦ / `waiting_approval`❗ / `error`⚠（アテンションのみ色＋微パルス）。
- 左レール accent は周辺視の補助として弱く残すか、ここで合わせて判断。
- 確定したら `docs/decisions.md` D3 の「未決」を更新（or 新 Dn）し、具体は `docs/status-card-design.md` へ。
