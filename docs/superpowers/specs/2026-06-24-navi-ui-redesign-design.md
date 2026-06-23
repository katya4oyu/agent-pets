# navi UI 刷新 ＋ 見た目デバッグ環境（playground / Cloudflare Workers）設計

- 日付: 2026-06-24
- 対象リポジトリ: katya4oyu/agent-pets（navi）

## 1. 背景と目的

navi（旧 agent-pets）は Codex / Claude Code / GitHub Copilot の状態をデスクトップペットで可視化する Tauri アプリ。現状フロントの UI が「ひどい」ため刷新する。

- スプライトは canvas + `setTimeout` の手描きで、アニメ定義（行番号）は `main.ts` にハードコード。
- 吹き出しはマジックナンバーだらけのレイアウト計算で、尻尾もない。
- ユーザーが「使い勝手がよかった」と評価した別リポジトリ codex-pet-web の Web Component（`<codex-pet>`）の方式を取り込みたい。
- Tauri（Rust ビルド）を起動せずに見た目をデバッグできる環境が欲しい。さらにそれを Cloudflare で共有・確認できるようにする。

## 2. スコープ

対象:
- フロント表示層（`app/src`）の刷新。
- 見た目デバッグ環境（Vite playground）の新設。
- playground の Cloudflare Workers 公開。

非対象（今回触らない）:
- Rust バックエンド（HTTP サーバー / hook CLI / 状態正規化 / マルチセッション状態管理）。プロトコルは維持。
- Tauri / Rust のビルドや配布。
- 対応エージェントの拡張（Codex / Claude Code / Copilot の 3 つのみ。汎用化はしない）。

## 3. アーキテクチャ

コンポーネント構成（ウィンドウは透過・装飾なし・最前面を維持）:

```
navi-shell（main.ts）
├─ <navi-pet>        … codex-pet.ts ベースの純TS Web Component
│     ・スプライト表示：CSS background-position + requestAnimationFrame
│     ・ドラッグ移動／右下リサイズを内蔵
│     ・state 属性に AgentState を渡す → 内部で pet.json のアニメへ変換
├─ 吹き出しスタック   … navi 固有・マルチセッション（縦積み、各枚にソースバッジ）
├─ セッションカウント／表示トグル
└─ 設定ボタン（⚙）
```

- `<navi-pet>` はペット本体（描画＋ドラッグ＋リサイズ）のみ担当。codex-pet-web の単一 `say()` は使わず、マルチセッション吹き出しは navi シェル側で実装する。
- **MoonBit は持ち込まない。** codex-pet-web の MoonBit は JS ターゲットの軽量な状態計算のみで、体感のパフォーマンス（滑らかさ）は描画方式（CSS background-position + rAF）由来。状態計算（tick / should_advance / choose_animation 相当）は純 TS に書き直す。

データフロー（Rust 側は不変）:

```
Rust（HTTP / hook / 状態正規化）→ "agent-state-changed"
        ↓
navi-shell が sessions Map を更新
        ├→ 吹き出しスタック更新（セッションごとに1枚）
        └→ 最優先 state を <navi-pet> の state 属性へ → アニメ切替
```

pet.json のマニフェスト化:
- 現状 `mio/pet.json` は画像パスのみ。codex-pet-web 規約で `columns / rows / frameWidth / frameHeight / animations` を追記する。
- アニメ名は codex-pet-web 互換（idle / running / waving / failed / review …）。navi 側で `AgentState → アニメ名` の対応表を持つ。
- これにより mio は navi でも codex-pet-web でも動く（アセットが両対応）。
- 状態 → アニメ対応（現状を継承）: `done=idle` / `running=running` / `waiting_approval=waving` / `error=failed` / `thinking=review`。`editing` のみ未決定（暫定で thinking と共用、フェーズ2で確定）。

## 4. 開発環境（Vite playground）

- Storybook は使わない（生 Web Component には過剰）。Vite の追加エントリで playground を構築する。
- Tauri 依存（`invoke` / `listen`）を `bridge.ts` に集約。本番＝実 Tauri、playground＝モック。これは playground のためだけでなく navi 本体の疎結合にもなる。

ディレクトリ:
```
app/
├── index.html          … 本番（Tauri）エントリ
├── playground.html     … 開発用エントリ（新規）
├── public/pets/mio/    … デバッグ用 mio スプライト
└── src/
    ├── bridge.ts       … Tauri 抽象（新規）
    ├── main.ts         … 本番ブートストラップ
    ├── playground.ts   … 開発用：モックブリッジ＋操作パネル（新規）
    ├── pet/            … <navi-pet>
    └── shell/          … 吹き出しスタック・バッジ・操作系
```

操作パネルで再現できること:
- 各 AgentState（thinking / running / editing / waiting_approval / done / error）→ ペットのアニメ
- source 切替（claude-code / codex / copilot）→ バッジ
- セッション追加 / 削除 → マルチセッション吹き出し
- 長文メッセージ / 長いプロジェクト名 → オーバーフロー
- ペットサイズ・吹き出し表示モード

## 5. Cloudflare Workers 公開

方式: **Cloudflare Workers Static Assets**（Cloudflare が Pages を Workers に一本化したため Workers で統一）。**Tauri / Rust は一切ビルドしない**（Vite の playground ビルドのみ）。

リポジトリ側に新規追加:
- `app/wrangler.jsonc` — `./dist` を静的配信。
- `app/playground.html` ＋ Vite 設定 — 公開ビルドでは playground を `/`（index.html）として出力。
- `bridge.ts` のモック — Tauri 不在のブラウザで動作。
- mio スプライトを公開ビルドに同梱。

Cloudflare ダッシュボード設定（Workers Builds）:
- Project name: `agent-pets`
- **Root directory: `app`**（必須。pnpm の依存・コードが `app/` 配下のため。ルート実行だと install が空振りする）
- **Build command: `pnpm install && pnpm run build:web`**（playground のみを出力する新スクリプト。`pnpm run build` は Tauri 版を出すので使わない）
- Deploy command: `npx wrangler deploy`（production）
- Non-production branch deploy command: `npx wrangler versions upload`（preview）
- Builds for non-production branches: ON

注: `build:web` は playground を `/` に出す専用スクリプトとして `app/package.json` に追加する。Rust / cargo / tauri build は呼ばない。

## 6. 見た目の作り込み（フェーズ2）

環境（フェーズ1）構築後、playground 上で iterate する:
- 吹き出しデザイン（尻尾・影・配色・複数セッション時の見せ方）
- 全体レイアウト・操作系（リサイズ／設定／トグル／セッションカウントの整理）
- ウィンドウサイズの妥当性
- 方針: codex-pet-web の見た目・操作感に寄せつつ navi 固有要件を満たす。詳細は環境上で見ながら決定する。

## 7. 実装順序

フェーズ1（環境）:
1. `bridge.ts` 抽出（invoke / listen を集約）、`src` を `pet/` `shell/` に再編。
2. `<navi-pet>` 移植（codex-pet.ts ベース、純 TS 化、CSS background-position + rAF）。
3. `mio/pet.json` マニフェスト化、`public/pets/mio/` 配置。
4. `playground.html` / `playground.ts` / 操作パネル。
5. `vite.config.ts`（マルチエントリ／公開ビルドで playground を `/`）、`build:web` スクリプト、`wrangler.jsonc`。
6. Cloudflare 接続（ユーザー操作：Root directory・Build command 設定）。

フェーズ2（見た目）:
7. playground 上で吹き出し・レイアウト・操作系を作り込み。

## 8. 確認事項・リスク

- mio スプライト（AI 生成キャラ）を Cloudflare で公開して問題ないか（codex-pet-web は aomi を public に置いて公開済みなので同様と想定）。
- Cloudflare 公開 URL は認証なし。playground はモックデータのみで機密はない。
- 既存の未コミット作業（navi リネーム等）がツリーにあるため、本 doc 以降の commit / ブランチ運用は要確認。
