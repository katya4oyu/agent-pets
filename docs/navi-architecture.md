# Navi アーキテクチャ設計書

> Status: **Draft / Vision** — 2026-06-17
> Working codename: **Navi**（現リポジトリ名 `agent-pets` のまま据え置き。リネームは後述の移行計画で実施）
> Scope: 設計のみ。本ドキュメントの時点ではコード変更を伴わない。

## 0. このドキュメントの位置づけ

`agent-pets` は現在、「コーディングエージェント（Codex / Claude Code / GitHub Copilot CLI）の
ライフサイクル hook を受け取り、デスクトップ常駐のペットとして状態を可視化する」**受動的な通知ビューア**である。

本設計書は、これを **ロックマンEXE のネットナビ** のような

- デスクトップに常駐し、
- 人格を持ち、
- **スキル（チップ）をモジュラーに着脱でき**、
- **自律的に振る舞い（移動・先回り・要約・通知への対応）**、
- 必要なら**既存のコーディングエージェントを「操縦」して重い仕事を任せる**

**能動的なオペレーター・コンパニオン**「Navi」へ進化させるための、土台となる設計とロードマップを定義する。

メタファの対応:

| ロックマンEXE | Navi |
| --- | --- |
| ネットオペレーター（人間） | ユーザー |
| ネットナビ | Navi 本体（常駐アバター + オペレーターコア） |
| バトルチップ / プログラム | **Skill（プラグイン）** |
| チップフォルダ / スロット | **Capability（権限）スロット** |
| ジャックイン先・重い戦闘 | **Agent Backend**（Claude Code / Codex / Copilot / HermesAgent …） |
| ネットの世界を動き回る | **自律的なデスクトップ常駐・移動** |

---

## 1. 設計原則

1. **オペレーター・ファースト**: Navi 自身は重い実行（コード生成・大規模編集）を担わない。
   判断・調整・要約・通知を行い、重い作業は **Agent Backend** に委譲（操縦）する。
2. **ルール優先 / LLM はフォールバック**: 自律判断はまず安価で決定的なルールで処理し、
   ルールで決められないときだけ **Reasoner（Local/Remote LLM）** に相談する。LLM は既定で OFF にできる。
3. **能力ベースのプラグイン（capability-gated）**: Skill は宣言した Capability の範囲でしか動けない。
   チップのように「与えた力だけ使える」を徹底し、サードパーティ拡張でも安全を保つ。
4. **ノンブロッキング & ロスレスでない通知**: hook 連携は今と同様、エージェントを絶対にブロックしない。
   状態更新は意図的に lossy。Navi がいなくても各エージェントは普通に動く。
5. **ローカルファースト / プライバシー**: すべて localhost に閉じる。外部送信（LLM API 等）は明示的なオプトイン。
   秘密情報（API キー）は OS のシークレットストアまたは権限を絞ったファイルに保存。
6. **後方互換と段階移行**: 既存の通知機能・hook・設定（`~/.agent-pets`）を壊さない。
   `Skill` 化・`Navi` 化はすべて非破壊で積み上げ、リネームは最後にまとめて行う。

---

## 2. 現状アーキテクチャ（agent-pets, as-is）

```text
Codex / Claude Code / Copilot CLI
        │  lifecycle hook → stdin JSON
        ▼
agent-pets-hook  (Rust CLI, src/bin/agent_pets_hook.rs)
        │  POST /events/<source>  (timeout ~150ms, fire-and-forget)
        ▼
Tauri backend  (src-tauri/src/lib.rs)
   ├─ tiny_http サーバ（空きポート → ~/.agent-pets/port）
   ├─ normalize(): payload → HookEvent { source, state, label, message, … }
   └─ app_handle.emit("agent-state-changed", HookEvent)
        ▼
Frontend  (src/main.ts)
   ├─ sessions: Map<source:session_id, {state, bubble}>
   ├─ 最優先 state でスプライト行を切替
   └─ セッションごとに吹き出し表示
```

特徴と限界:

- **一方向**: エージェント → ペット の通知のみ。Navi 側から働きかける経路がない。
- **状態がフロントエンドに散在**: セッション状態は `main.ts` の `sessions` Map に保持され、永続化も
  バックエンド集約もない。自律判断や横断的な記憶に使えない。
- **拡張点がない**: 新しい振る舞い・スキルを足すには `lib.rs` / `main.ts` を直接編集するしかない。
- **固定位置**: ペットはユーザーがドラッグした位置に静止。自律的な移動・反応はない。

---

## 3. 目標アーキテクチャ（Navi, to-be）

6 層構成。既存資産（hook 受信・正規化・スプライト表現）は **層の一部として再利用** する。

```text
┌──────────────────────────────────────────────────────────────────────┐
│ 6. Avatar / Presentation 層   自律移動・チャットUI・スキルUI・吹き出し      │  (frontend)
├──────────────────────────────────────────────────────────────────────┤
│ 5. Skill / Plugin 層 (= チップ)   manifest + capability で着脱する振る舞い │
├──────────────────────────────────────────────────────────────────────┤
│ 4. Operator Core 層   Sense→Decide→Act の自律ループ / Reasoner 連携       │  ← NEW
├──────────────────────────────────────────────────────────────────────┤
│ 3. World Model / Memory 層   全セッション状態・履歴・プロジェクト文脈・記憶  │  ← NEW(集約)
├──────────────────────────────────────────────────────────────────────┤
│ 2. Event Bus 層   /events 受信 + 内部イベント(タイマー/チャット/スキル出力) │  ← 一般化
├──────────────────────────────────────────────────────────────────────┤
│ 1. Connector 層   Inbound(hook受信) + Outbound(エージェント操縦)          │  ← Outbound NEW
└──────────────────────────────────────────────────────────────────────┘
        ▲ inbound hooks                         ▼ outbound dispatch
   Codex / Claude Code / Copilot          Claude Code / Codex / HermesAgent / …
```

各層の責務とデータの流れ:

- **下から上（センス）**: Connector(1) が外部イベントを受け、Event Bus(2) に流す。
  World Model(3) が状態を更新し、Operator Core(4) と Skill(5) が反応、Avatar(6) が表現する。
- **上から下（アクト）**: ユーザー操作・自律判断・Skill が **Action** を発行し、
  通知/アニメ/移動（→ Avatar）や、エージェントへの **dispatch**（→ Connector outbound）に変換される。

---

## 4. Event Bus 層（イベントバス）

現在の `POST /events/<source>` を、Navi 内部の汎用イベントバスへ一般化する。

- **外部イベント（Inbound）**: 既存の hook イベント。`HookEvent` は `AgentEvent::Hook(...)` の一種になる。
- **内部イベント**: タイマー（tick / idle 検出）、ユーザーチャット入力、Skill の出力、
  Connector からの dispatch 結果、Reasoner の応答など。

```rust
// 例（イラスト用・確定仕様ではない）
enum NaviEvent {
    Hook(HookEvent),                 // 既存。外部エージェントのライフサイクル
    Tick { at: Instant },            // 自律ループの心拍
    UserMessage { text: String },    // ユーザー → Navi のチャット
    SkillOutput { skill: SkillId, payload: Value },
    AgentReply { backend: BackendId, session: String, payload: Value },
}
```

- バスは backend 側に集約する（現状フロントエンドの `sessions` を移管）。
- 各イベントは World Model 更新と、購読している Skill / Operator Core への配信を行う。
- 配送はベストエフォート（ロスレスでない原則を維持）。

---

## 5. World Model / Memory 層（状態・記憶）

自律判断の根拠となる「世界の現在像」と「記憶」をバックエンドに集約する。

- **World Model（揮発・現在状態）**:
  - アクティブな全セッション（`source` × `session_id`）と最新 `AgentState`・ラベル・cwd・project。
  - 集約状態（最優先 state、エラー数、承認待ち件数、最終アクティビティ時刻）。
  - これは現在 `main.ts` の `sessions` Map / `getHighestPriorityState()` が担っている責務の移管。
- **Memory（永続）**:
  - `~/.agent-pets/`（将来 `~/.navi/`）配下に、セッション履歴・ユーザー設定・Skill ごとの
    スコープ付きストレージ・短期/長期メモを保存。
  - 用途: 「同じプロジェクトでまた承認待ちが続いている」等の文脈的判断、チャットの継続性、
    Skill の状態保持。

設計上のポイント: フロントエンドは World Model の **ビュー** に徹し、状態の真実はバックエンドに置く。
これにより、ウィンドウ非表示でも自律ループが回り、複数ウィンドウ/アバターも一貫した状態を共有できる。

---

## 6. Operator Core 層（自律ループ）

Navi の「中核」。**Sense → Decide → Act** を、tick（定期）とイベント駆動の両方で回す。

```text
   ┌─────────── イベント / Tick ───────────┐
   ▼                                       │
 Sense    World Model + 新着イベントを観測   │
   ▼                                       │
 Decide   1) 購読 Skill のルールを評価        │
          2) 決まらなければ Reasoner に相談    │  (任意・オプトイン)
   ▼                                       │
 Act      Action を発行                      │
          - Avatar: 通知 / アニメ / 移動 / 発話 │
          - Connector(outbound): エージェントへ dispatch │
          - Memory: 記録                      │
   └───────────────────────────────────────┘
```

- **ルール優先**: ほとんどの反応（承認待ちでこちらを向く、エラーで駆け寄って知らせる、
  完了で落ち着く、長時間アイドルで休む）はルール/ステートマシンで安価に処理。
- **Reasoner フォールバック**: 「この通知をどの backend に振るべきか」「複数の出来事をどう一言で要約するか」
  「いま声をかけるべきか/邪魔しないべきか」など、曖昧な判断のみ LLM に委譲（§7）。
- **自律ふるまい（autonomy）**: ペットの移動・徘徊・視線・気分（mood）も Operator Core が
  Action として駆動する（固定位置からの脱却）。

> 重要原則の再掲: Operator Core は「**何を・誰に任せ・どう伝えるか**」を決める頭脳であって、
> 重いコード作業そのものは行わない。実作業は Connector(outbound) 経由で Agent Backend に委譲する。

---

## 7. Reasoner 層（Local / Remote LLM）

オプトインの推論バックエンド。**OpenAI / Claude 互換 API** を叩く薄いクライアントとして実装し、
ローカル LLM（Ollama / LM Studio / llama.cpp の OpenAI 互換サーバ等）でもホスト型でも差し替え可能にする。

用途（いずれも軽量・短コンテキスト）:

- ユーザーとの自然言語チャット（Navi への話しかけ）。
- 複数イベントの一言要約（吹き出し用）。
- ルーティング判断（「この依頼は Claude Code か HermesAgent か」）。
- 自律ふるまいの“さじ加減”（声をかける/待つ、mood の選択）。

設計方針:

- **既定 OFF**: API 未設定なら Reasoner なしでも全機能のコア（ルールベース）は動く。
- **プロバイダ抽象**: `Reasoner { complete(prompt, opts) }` の単一インターフェイス。`base_url` / `model` /
  `api_key` を設定で切替（OpenAI 互換 / Anthropic 互換 / ローカル）。
- **コスト/レイテンシ管理**: 呼び出しは間引き（デバウンス・キャッシュ・最大頻度）。重い思考はしない。
- **鍵の扱い**: API キーは OS シークレットストア優先、なければ権限を絞った設定ファイル。ログに残さない。

> 注: 「Navi が直接 LLM で重い実装をする」用途は対象外。それは Agent Backend（§8）の役割。

---

## 8. Connector 層（コネクタ / エージェント操縦）

外部エージェントとの境界。**Inbound（観測）** と **Outbound（操縦）** の双方向を扱う。

### 8.1 Inbound（既存・観測）

- 現状の hook 受信そのもの。`agent-pets-hook` CLI → `/events/<source>` → normalize → `HookEvent`。
- Connector はこれを「読み取り専用の観測チャネル」として保持し続ける（後方互換）。

### 8.2 Outbound（新規・操縦）

Navi から外部エージェントへ**仕事を投げる**経路。これが「オペレーター化」の核心。

```rust
// 例（イラスト用）
trait AgentConnector {
    fn id(&self) -> BackendId;                 // "claude-code" | "codex" | "hermes" | …
    fn capabilities(&self) -> ConnectorCaps;   // inbound_only? dispatch? stream? cancel?
    fn dispatch(&self, task: AgentTask) -> Result<DispatchHandle>; // 仕事を渡す
    fn status(&self, handle: &DispatchHandle) -> AgentStatus;       // 監視
    fn cancel(&self, handle: &DispatchHandle) -> Result<()>;
}
```

- **Backend はプラグイン的に追加可能**: Claude Code / Codex / Copilot に加え、
  ユーザーの想定する **HermesAgent** のような重い実行系も 1 つの Connector として登録する。
- **dispatch 手段は backend ごとに異なる**: CLI 非対話起動、対話セッションへの入力、
  ローカル RPC、専用 API など。Connector がその差分を吸収する。
- **能力宣言**: ある backend は inbound のみ（hook だけ）、別の backend は dispatch + 監視 + キャンセルまで、
  と `ConnectorCaps` で表明する。Operator Core はこれを見てルーティングする。

この層により Navi は「通知を見る」から「**気づいて、適切な相棒に振り、見届ける**」へ進化する。

---

## 9. Skill / Plugin 層（スキル = チップ）

Navi のモジュラー性の心臓部。機能・振る舞いは **Skill** として着脱する。

### 9.1 Skill マニフェスト

各 Skill は宣言的なマニフェスト（TOML/JSON）を持つ:

```toml
# 例（イラスト用）
id = "approval-nudge"
display_name = "Approval Nudge"
description = "承認待ちが続いたらアバターが知らせる"
version = "0.1.0"
entrypoint = "builtin"          # builtin | process | wasm
subscribes = ["Hook", "Tick"]   # 購読するイベント種別
capabilities = ["events:read", "avatar:control", "notify"]  # 必要な権限のみ宣言
[config]                         # 任意のユーザー設定スキーマ
idle_seconds = 30
```

### 9.2 SkillContext（Skill に渡す API）

Skill は宿主（host）から、宣言した Capability の範囲に絞られた `SkillContext` を受け取る:

- `ctx.subscribe(event_kind)` / `ctx.on_event(...)` — イベント購読
- `ctx.world()` — World Model の読み取り（スコープ付き）
- `ctx.emit(action)` — Action 発行（通知・アニメ・移動・発話・dispatch）
- `ctx.reasoner()` — Reasoner 呼び出し（`net`/`reasoner` capability がある場合のみ）
- `ctx.store()` — Skill ごとのスコープ付き永続ストレージ
- `ctx.ui()` — アバター/パネルへの UI サーフェス提供（任意）

### 9.3 Capability（権限スロット）

「与えた力だけ使える」をエンジンレベルで強制する。例:

| Capability | 内容 |
| --- | --- |
| `events:read` | イベント/World Model の読み取り |
| `avatar:control` | アニメ・移動・吹き出し・発話 |
| `notify` | OS 通知の送出 |
| `agent:dispatch` | Outbound Connector 経由でエージェントに仕事を投げる |
| `reasoner` | LLM 推論の利用 |
| `net` | 任意の外部ネットワーク（既定で要承認） |
| `fs:read` / `fs:write` | ファイルアクセス（パススコープ付き） |

未宣言 Capability の API は `SkillContext` に露出しない／呼んでも拒否される。

### 9.4 ロード方式（段階的に強化）

| Tier | 形態 | 特性 | 想定時期 |
| --- | --- | --- | --- |
| Tier 1 | **Builtin（Rust trait object）** | 同一バイナリ内で静的登録。最速・最安全。まずここから | 初期 |
| Tier 2 | **External process** | 別プロセス + ローカル RPC(JSON-RPC over stdio/HTTP)。言語非依存 | 中期 |
| Tier 3 | **WASM**（wasmtime / extism 等） | サンドボックス化。サードパーティ配布向け | 後期 |

- まず既存の通知機能を **Builtin Skill「StatusBubble」** として再実装し、振る舞いを Skill 化する道を作る。
- レジストリは pets と同様に `~/.navi/skills/<id>/` で発見可能にする（将来のマーケットプレイス前提）。

---

## 10. Avatar / Presentation 層（アバターと自律表現）

現在のスプライト表現を拡張し、Navi に「生きている」存在感を与える。

- **自律移動**: 固定位置をやめ、Operator Core の Action で徘徊・反応・接近（用件があるとき寄ってくる）。
- **既存スプライト資産の活用**: 9 行アトラス（idle/running/waving/jumping/failed/waiting/review …,
  `docs/codex-pet-spritesheets.md`）はそのまま使え、`running-left/right`・`jumping` 等の未使用行を
  移動表現に割り当てられる。
- **チャット UI**: ユーザーが Navi に話しかける入力。Reasoner / Skill が応答。
- **Skill UI サーフェス**: Skill が小さなパネル/メニューを提供できる（チップ的な拡張 UI）。
- **マルチウィンドウ**: アバター本体・チャット・設定を分離可能に。状態は World Model から供給。

表現は「ビュー」。真実の状態はバックエンド（World Model）にあるため、表示の有無に関わらず Navi は動く。

---

## 11. セキュリティ / 権限 / プライバシー

- **ローカル境界**: HTTP サーバは `127.0.0.1` のみ。Outbound dispatch・LLM 以外で外部送信しない。
- **Capability ゲーティング**: Skill は宣言した権限の範囲でのみ動作（§9.3）。`net` / `fs:write` /
  `agent:dispatch` は既定で要承認。
- **Tier 別サンドボックス**: Builtin は信頼前提、External process はプロセス分離、WASM は強サンドボックス。
- **dispatch の確認**: Navi がエージェントに仕事を投げる（特に破壊的操作）前には、ポリシーに応じて
  ユーザー確認を挟める仕組みを用意する。
- **秘密情報**: LLM/API キーは OS シークレットストア優先。ログ・テレメトリに残さない。
- **オプトイン**: Reasoner・Outbound・サードパーティ Skill はいずれも明示的な有効化が必要。

---

## 12. 設定とディレクトリ構成

当面は `~/.agent-pets/` を維持し、Navi 化のタイミングで `~/.navi/` へ移行（旧パスを後方互換で読む）。

```text
~/.agent-pets/            (将来: ~/.navi/)
├── port                  既存。HTTP サーバのポート
├── bin/agent-pets        既存。hook CLI（将来: bin/navi）
├── config.toml           NEW: Navi 全体設定（Reasoner, backends, autonomy）
├── state/                NEW: World Model のスナップショット/履歴
├── memory/               NEW: 長期メモ
└── skills/<id>/          NEW: Skill とそのスコープ付きストレージ

~/.codex/pets/<id>/       既存。スプライト資産（互換維持）
```

---

## 13. 現状コード → 目標アーキテクチャ 対応表

| 現状 (agent-pets) | 目標 (Navi) での位置づけ |
| --- | --- |
| `agent_pets_hook.rs`（hook CLI） | Connector(1) **Inbound** チャネル。そのまま維持 |
| `lib.rs` の `tiny_http` サーバ / `/events/<source>` | Event Bus(2) の Inbound 入口に一般化 |
| `lib.rs` の `normalize()` / `HookEvent` / `AgentState` | Event Bus(2) の正規化。`HookEvent` は `NaviEvent::Hook` の一種 |
| `main.ts` の `sessions` Map / `getHighestPriorityState()` | World Model(3) に移管（バックエンド集約） |
| `main.ts` のスプライト/吹き出し描画 | Avatar(6)。World Model のビューに徹する |
| `lib.rs` の `setup_hooks()` / tray メニュー | Connector(1) 設定 + Skill/設定 UI へ整理 |
| （新規） | Operator Core(4), Reasoner(7), Outbound Connector(8), Skill host(9) |

---

## 14. リネーム / 移行計画（後で実施・今回は据え置き）

ユーザー決定: **当面 `agent-pets` のまま据え置き**。本書で `Navi` を目標名として確定し、移行はまとめて行う。

移行手順（将来）:

1. 内部モジュール名を中立化し、機械的にリネーム可能な構造にしておく。
2. プロダクト名 `Navi`、CLI `navi`、設定ディレクトリ `~/.navi`（旧 `~/.agent-pets` を後方互換で読む）へ切替。
3. リポジトリ名・README・アイコン・トレイ表記を更新。
4. 互換シム期間を設け、旧 hook 設定（`agent-pets hook …`）も一定期間動かす。

> 候補名は `Navi` を第一候補とする。他案（Operator / Buddy / Familiar 等）を比較したい場合は別途検討。

---

## 15. オープンな論点

- **Outbound dispatch の現実解**: Claude Code / Codex を非対話で安全に起動・継続入力する標準手段。
  PTY 制御 / CLI フラグ / ローカル RPC のどれを一次対応にするか。
- **HermesAgent の連携 IF**: どのプロトコル（CLI / HTTP / MCP 等）で dispatch・監視するか。
- **Reasoner の既定プロバイダ**: ローカル（Ollama 等）を既定にするか、未設定 OFF を既定にするか。
- **Skill 配布**: Tier 2（process）と Tier 3（WASM）のどちらを先に正式サポートするか。
- **自律移動の煩わしさ制御**: 邪魔にならない徘徊頻度・集中モード（Do Not Disturb）の既定。
- **マルチデバイス/リモート**: 将来、リモートのエージェント活動も観測・操縦対象に含めるか。

---

## 関連ドキュメント

- ロードマップ: [`docs/navi-roadmap.md`](./navi-roadmap.md)
- hook 連携の現行設計: [`docs/superpowers/specs/2026-05-21-hook-integration-design.md`](./superpowers/specs/2026-05-21-hook-integration-design.md)
- スプライト仕様: [`docs/codex-pet-spritesheets.md`](./codex-pet-spritesheets.md)
- hook スキーマ調査: [`docs/hook-schema-research.md`](./hook-schema-research.md)
