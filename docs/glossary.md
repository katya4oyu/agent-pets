# 用語集 (Glossary)

> navi / agent-pets の **厳密な用語対応表**。会話・指示・レビュー・コードの**共通言語をここに一元化**する。
> 各項目の見出し語を**正式名**とし、通称・旧称は「別名/旧称」に退避する。曖昧な語（特に「吹き出し」）は、
> 本書で必ず正式名へ翻訳してから使うこと。
>
> 責務分担: **思想・背景** は `docs/concept.md`、**設計** は `docs/navi-architecture.md`、
> **名前と対応（だけ）** は本書。定義が衝突したら**本書を正**とする。
>
> 凡例: 「コード識別子」は現状の実体（CSS クラス / TS シンボル / ファイル）。リネーム予定があれば明記する。

---

## 1. 表示要素（アバター上に出るもの）

| 正式名 | 定義 | コード識別子（現状） | 別名/旧称 | これではない |
|---|---|---|---|---|
| **アバター** (avatar / pet) | 全セッションの集約状態を体現するキャラ本体。スプライトを描画し、ドラッグ／リサイズを持つ。 | `<navi-pet>`（`packages/ui/src/codex-pet/navi-pet.ts`）。app シェルは独自 canvas（`app/src/main.ts` の `.pet-sprite`） | ペット, pet, mio（mio は具体キャラ名） | 喋るキャラではない（発話 UI ではない） |
| **ステータスカード** (status card) | **1 エージェント・セッションの現在状態を表す 1 枚**。source / state / タイトル / message / cwd を含む。セッションが続く限り残り、その場で更新される。 | `.speech`（`app/src/styles.css`, `examples/playground`）, `createBubbleElement` / `updateBubbleElement`（`app/src/main.ts`）, `createBubble` / `updateBubble`（`examples/playground/src/navi-ui.ts`） | **吹き出し**, speech bubble, bubble, speech | セリフ／チャット発言ではない。トーストでもない（→ §4） |
| **ステータススタック** (status stack) | ステータスカードを縦に積む列。**最大 3 枚、超過はスクロール**（`issues/b3f2a1`）。 | `.speech-stack`（app）, `.navi-bubbles`（playground） | 吹き出しスタック, bubble stack | — |
| **ソースバッジ** (source badge) | カード上の、どのエージェント由来かを示すロゴ。`@lobehub/icons-static-svg` の実ロゴを `currentColor` で着色。 | `.source-badge`, `sourceConfig`（app / playground） | バッジ | — |
| **尻尾** (tail) | ステータスカードがアバターを指す三角の突起（callout のポインタ）。 | `.speech-tail`（playground。app には未実装） | しっぽ, pointer | — |
| **セッションカウント** (session count) | アクティブなセッション数。スタック非表示時にトグル上へ出る。 | `.session-count` | カウント | 通知バッジ（未読数）ではない |
| **表示トグル** (toggle) | スタックの表示/非表示を切り替えるボタン。 | `.bubble-toggle` | トグル | — |
| **リサイズハンドル** (resize handle) | アバターのサイズ変更つまみ（右下）。 | `.resize-handle` | — | — |
| **設定ボタン** (settings) | hook セットアップ等の操作（⚙）。 | `.setup-btn` | — | — |
| **ペット発話** (PetSpeech) | `<navi-pet>` 自身が持つ**単発・一時**の吹き出し（`duration` で自動消滅）。**ステータスカードとは別物**。 | `say()` / `clearSpeech()` / `speech` 属性 / `.speech`（navi-pet 内部・シャドウ DOM） | navi-pet の speech | ステータスカードではない（こちらは状態表示・連続更新） |

> ⚠️ **`.speech` 衝突注意**: 「ステータスカード（§1）」と「ペット発話（navi-pet 内部）」が**同じ `.speech`** を使っている。出自が違う別概念なので、会話では必ず**ステータスカード**／**ペット発話**で呼び分ける。コードのリネームは `c4b1e0` で解消予定。

---

## 2. 状態モデル (State model)

### state（`AgentState`）

セッションの現在状態。値は固定の 6 つ（`app/src/state.ts` の `AgentState`）。優先度 `STATE_PRIORITY`、表示名 `stateLabels`。

| state | 意味（hook 由来） | 表示名 (`stateLabels`) | 優先度 | 区分 |
|---|---|---|---|---|
| `error` | tool / hook 失敗で要注目 | Needs attention | 6 | **アテンション** |
| `waiting_approval` | 許可・入力待ち | Waiting approval | 5 | **アテンション** |
| `thinking` | プロンプト受領、計画/推論中 | Thinking | 4 | 進捗 |
| `running` | コマンド/ツール実行中 | Running | 3 | 進捗 |
| `editing` | ファイル編集中 | Editing | 2 | 進捗 |
| `done` | ターン完了 | Ready | 1 | 完了 |

| 正式名 | 定義 | コード識別子 |
|---|---|---|
| **アテンション** (attention request) | **人間の操作・判断を要する** state の部分集合 = `{ waiting_approval, error }`。唯一「通知」に相当する区分。 | （区分。§4 参照） |
| **最優先 state** (highest-priority state) | 全セッションの state を `STATE_PRIORITY` で比較した最大値。**アバターのアニメを決める**。セッションが無ければ `done`。 | `highestPriorityState()` |
| **表示モード** (speech mode) | スタックの可視性ポリシー。`show`=常時表示 / `hide`=常時非表示 / `auto`=最優先が `done` 以外のとき表示。 | `SpeechMode`, `isSpeechVisibleInAuto()` |

---

## 3. データ & パイプライン

| 正式名 | 定義 | コード識別子 |
|---|---|---|
| **source** | どのエージェントか。`claude-code` / `codex` / `copilot` の 3 つのみ。 | `HookEventPayload.source`, `sourceConfig` |
| **セッション** (session) | 1 つのエージェント実行単位。1 セッション = 1 ステータスカード。キーは `source:session_id`（無ければ source）。 | `sessionKey()`, `sessions` Map（`app/src/main.ts`） |
| **イベント** (event) | hook が送る状態変化 1 件。`POST /events/<source>` で受信。 | `HookEventPayload`, `"agent-state-changed"` |
| **navi-hook** | 各エージェントの hook から stdin を受け、上記イベントを POST する Rust CLI。 | `app/src-tauri/src/bin/navi_hook.rs` |

---

## 4. パターン分類（取り違え厳禁）

ステータスカードの「**本質**」を取り違えないための整理。

| パターン | 駆動 | 例 | navi との関係 |
|---|---|---|---|
| **ステータス表示 / Live Activity** | **状態駆動・連続**（「今こうである」をその場で更新、セッション終了で消える） | iOS Live Activity / Dynamic Island | **ステータスカードはこれ。** |
| トースト / 通知 (toast / notification) | **イベント駆動・離散**（「起きた」を点で知らせる） | macOS の Notification（Banner=一時 / Alert=永続） | 似て非なる。例外的に **アテンション（waiting_approval / error）だけ**が通知に相当 |
| callout / 吹き出し (speech balloon) | 形態の話（要素にアンカー＋尻尾） | ツールチップの親戚 | ステータスカードは callout **形態**を借りているが、中身は状態表示 |
| 発話 / チャット (speech / chat) | キャラの発言 | 会話 UI | **無関係**。アバターは喋らない（→ ペット発話 `say()` は別物） |

要点: **ステータスカード = アバターにアンカーした、セッション状態の Live Activity 表示**。「吹き出し」は通称、「トースト」は近縁だが別、「発話」は誤り。

---

## 5. 出自による責務の区別（codex 系の取り違え注意）

`docs/concept.md` の用語節と同じ。詳細・正は concept.md / `docs/codex-pet-spritesheets.md`。

| 用語 | 意味 |
|---|---|
| **codex 本家** | OpenAI Codex の pets 機能の**スプライト／アトラス規約**（`~/.codex/pets/<id>/`、8×9・192×208、行ごとに idle/running/.../review）。`codex-pet-web` のことではない。 |
| **codex-pet**（責務名） | 上記 codex 規約に依存する**描画・アニメ層**。コードは `packages/ui/src/codex-pet/`（`navi-pet` + `pet-core`）。 |
| **codex-pet-web** | リポジトリオーナー自作の**練習リポジトリ**。本家でも上流でもない。必要コードは navi へ移植済み・追従不要。 |
| **ui**（責務名） | navi / agent-pets **固有**の表現要件（ステータスカード・スタック・ソースバッジ・操作系・セッションカウント）。codex 由来ではない。 |
