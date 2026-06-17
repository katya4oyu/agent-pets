# Navi ロードマップ

> Status: **Draft / Vision** — 2026-06-17
> 対になる設計書: [`docs/navi-architecture.md`](./navi-architecture.md)

`agent-pets`（受動的通知ビューア）→ `Navi`（能動的オペレーター・コンパニオン）への段階移行計画。
**各フェーズは非破壊**で積み上げ、既存の通知機能・hook・設定を壊さないことを前提とする。

## 方針サマリ

- **頭脳**: オペレーター中心（既存エージェントを操縦）。重い実行は外部 Agent Backend（HermesAgent 等）へ委譲。
- **LLM**: 軽量・オプトイン。自律ふるまい・要約・ルーティング・チャットにのみ使用。
- **拡張**: Skill（チップ）＋ Capability（権限スロット）でモジュラーに機能追加。
- **リネーム**: 据え置き。安定後にまとめて `Navi` へ。

---

## フェーズ一覧（俯瞰）

| Phase | テーマ | 主な成果 | 破壊的変更 |
| --- | --- | --- | --- |
| 0 | ビジョン確定 | 本設計書＋ロードマップ | なし |
| 1 | 基盤化 | 状態のバックエンド集約 + Skill trait + Builtin レジストリ | なし（内部リファクタ） |
| 2 | 自律ループ | Operator Core + 先回り Skill 群 | なし |
| 3 | 操縦（Outbound） | Connector trait + 初の dispatch backend | なし（追加機能） |
| 4 | アバター自律化 | 自律移動・mood・チャット UI | なし（UI 追加） |
| 5 | プラグイン解放 | 外部/WASM Skill host + manifest + capability sandbox | なし（追加機能） |
| 6 | Navi 化 | リネーム・設定移行・マーケットプレイス | あり（互換シム付き） |

---

## Phase 0 — ビジョン確定（完了物 = 本書）

- **目的**: 目標アーキテクチャと用語（Navi / Skill / Connector / Operator Core / Reasoner）を確定。
- **成果物**: `docs/navi-architecture.md`, `docs/navi-roadmap.md`。
- **完了条件**: 方針（オペレーター中心 / LLM はオプトイン / リネーム据え置き）に合意。

---

## Phase 1 — 基盤化（Foundation）

> 自律性・プラグイン性の土台を、ユーザー体験を一切変えずに用意する。

- **目的**: 状態の真実をバックエンドへ集約し、振る舞いを「Skill」として差せる構造を作る。
- **作業**:
  1. **World Model のバックエンド集約**: `main.ts` の `sessions` / `getHighestPriorityState()` 相当を
     Rust 側へ移し、フロントエンドはそのビューにする。`agent-state-changed` は集約状態も配信。
  2. **イベントの内部表現を一般化**: `HookEvent` を内部 `NaviEvent` の一種として扱えるようにする。
  3. **Builtin `Skill` trait と登録機構**: `Skill { id, subscribes, on_event(ctx, event) }` を定義し、
     静的レジストリで登録・配信する host を実装。
  4. **既存通知を Skill 化**: 現行の「state→アニメ＋吹き出し」を Builtin Skill **`StatusBubble`** として再実装し、
     振る舞いが Skill 経由で成立する経路を実証。
- **完了条件**: 見た目・通知挙動は現状と同一のまま、内部が「World Model + Skill host + StatusBubble」で動く。
  既存テスト（normalize / tray / hook 登録）が全て緑。
- **非破壊保証**: hook・`~/.agent-pets`・tray・スプライトの外部仕様は不変。

---

## Phase 2 — 自律ループ（Operator Core）

- **目的**: 受動から能動へ。tick + イベント駆動の Sense→Decide→Act ループを導入。
- **作業**:
  1. **Operator Core**: tick（心拍）と idle 検出、ルール優先の判断エンジン。
  2. **先回り Skill 群（Builtin, ルールベース）** の例:
     - `ApprovalNudge`: 承認待ちが続いたらアバターが知らせる。
     - `IdleRest`: 長時間アイドルで落ち着く/休む。
     - `ErrorAlert`: エラー発生時に駆け寄って通知。
     - `ActivityDigest`: 直近の出来事を一言に要約（まずはテンプレ、後で Reasoner）。
  3. **Reasoner クライアント（オプトイン・既定 OFF）**: OpenAI/Claude 互換 API の薄いラッパ。
     `ActivityDigest` 等の曖昧判断のフォールバックに使用。鍵はシークレットストア。
- **完了条件**: LLM 未設定でもルールベースの先回り通知が動く。設定すれば要約が自然文になる。
- **非破壊保証**: 自律ふるまいは設定で OFF にでき、OFF 時は Phase 1 と同等の挙動。

---

## Phase 3 — 操縦（Outbound Connector）

- **目的**: Navi が「気づく」だけでなく「適切な相棒に仕事を振り、見届ける」。
- **作業**:
  1. **`AgentConnector` trait**（dispatch / status / cancel / capabilities）を定義。
  2. **初の Outbound backend を 1 つ実装**（例: Claude Code もしくは Codex の非対話起動）。
     まずは「Navi のチャット入力 → 選んだ backend に prompt を投げ、進捗を World Model に反映」。
  3. **HermesAgent コネクタ**（重い実行委譲先）を 1st クラスの backend として接続。
  4. **`agent:dispatch` capability** と dispatch 前のユーザー確認ポリシー。
- **完了条件**: ユーザーが Navi 経由で 1 つの backend に仕事を投げ、状態が可視化される。
- **オープン論点**: 非対話起動/継続入力の標準手段（PTY / CLI フラグ / ローカル RPC / MCP）。

---

## Phase 4 — アバター自律化（Avatar & Autonomy）

- **目的**: 固定位置の脱却。Navi に存在感と対話性を与える。
- **作業**:
  1. **自律移動**: Operator Core の Action で徘徊・接近・反応。未使用スプライト行（running-left/right,
     jumping）を移動表現へ割当。
  2. **mood / 状態表現**の拡充。
  3. **チャット UI**: ユーザー → Navi の自然言語入力と応答（Reasoner / Skill 連携）。
  4. **Do Not Disturb / 集中モード**と徘徊頻度の既定調整（煩わしさ制御）。
- **完了条件**: Navi が自律的に動き、話しかけて応答が返る。邪魔にならない既定。

---

## Phase 5 — プラグイン解放（Third-party Skills）

- **目的**: サードパーティが安全に Skill を足せる「チップ」エコシステム。
- **作業**:
  1. **Skill マニフェスト**（id/version/subscribes/capabilities/config）正式化。
  2. **External process Skill host**（Tier 2, JSON-RPC over stdio/HTTP）。
  3. **WASM Skill host**（Tier 3, wasmtime/extism 等）でサンドボックス配布対応。
  4. **Capability サンドボックス**の強制（`net`/`fs`/`agent:dispatch` の承認フロー）。
  5. `~/.navi/skills/<id>/` レジストリと一覧 UI。
- **完了条件**: 外部 Skill を 1 つインストール → capability 承認 → 動作、を一通り実証。

---

## Phase 6 — Navi 化（Rename & Release）

- **目的**: 安定後にブランドと配布を `Navi` へ。
- **作業**:
  1. プロダクト名 `Navi` / CLI `navi` / 設定 `~/.navi`（旧 `~/.agent-pets` を後方互換で読む）。
  2. リポジトリ名・README・アイコン・tray 表記の更新。
  3. 旧 hook 設定の互換シム期間。
  4. Skill マーケットプレイス/レジストリの整備。
- **完了条件**: 既存ユーザーが設定移行なしで継続利用でき、新規は `Navi` として導入できる。

---

## 直近の着手候補（Phase 1 の最初のタスク）

設計合意後、最初に着手するなら以下が安全かつ効果的:

1. World Model を Rust 側に新設し、`agent-state-changed` に集約状態フィールドを追加（フロントは互換維持）。
2. `Skill` trait + Builtin レジストリの骨組みを追加（まだ振る舞いは移さない）。
3. 既存の通知ロジックを Builtin Skill `StatusBubble` として切り出し、host 経由で配信。

いずれも外部仕様を変えない内部リファクタで、既存テストで回帰を防げる。
