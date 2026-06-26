# ステータスカードの視覚デザイン（具体仕様・適用値）

> **これは具体ドキュメント**（事例＝ステータスカードの実装値・式・コード識別子）。
> **思想・原則の正は `docs/design-principles.md`**（身体性を背骨にした P1–P5）。本書はその**最初の適用例**で、
> 各判断には対応する原則（P1…）を併記する。新 UI は原則側から導き、結果を本書形式でここへ追記する。
>
> playground（`examples/playground`）で詰めた **ステータスカード**まわりの設計判断を、
> 「なぜその値・その挙動なのか」まで含めて記録する。値の確定は playground、用語の正は
> `docs/glossary.md`、思想の背景は `docs/concept.md`。
>
> 対象コード:
> - `examples/playground/src/playground.css` … 実際の `.status-card` / `.source-badge` スタイルと CSS 変数。
> - `examples/playground/src/playground.ts` … パラメータ露出（スライダー/トグル）と動的計算（`applyShadowVector()`）。
> - `packages/ui/src/navi/source-badge.ts` … ソースバッジのロゴ供給とバリアント定義。
>
> ここで確定した既定値は、後で `@navi/ui` のコンポーネント既定／app シェルへ焼き込む前提
> （抽出 `c4b1e0`、シェル移行 `e1f5c3`）。playground は「動くシェルの量産」ではなく
> **design とパラメータを確定するサンドボックス**（`issues/d9a2f7`）。

---

## 0. 大前提：ステータスカードとは何か

`docs/glossary.md` の通り、**ステータスカード = アバターにアンカーした、1 エージェント・セッションの状態を表す
Live Activity 表示**。発話でもトーストでもない。

- **状態駆動・連続**：「今こうである」をその場で更新する。**自動消滅しない**＝手動 dismiss まで残す（`docs/decisions.md` D1）。
- 形態としてはアバターにアンカーする callout を借りるが、**尻尾は廃止**。中身は状態表示。

この性格が以下すべての判断の土台。「キャラのセリフ」ではなく「OS の Live Activity に近い情報レイヤー」
として、**静かで・整っていて・物理的に納得できる**見え方を目指す。

---

## 1. ソースバッジ

カード左上の、どのエージェント由来かを示すロゴ（`.source-badge`）。

### 1.1 配色バリアント：mono / tint / brand

| バリアント | 意味 | 着色 |
|---|---|---|
| **mono**（既定） | モノクロ。**全ソース共通の中立グレー**で統一トーン。 | `--badge-mono`（既定 `#52525b`） |
| **tint** | 単色。ソースごとのブランド色。 | `--src-claude-code` / `--src-codex` / `--src-copilot` |
| **brand** | 公式配色そのまま。再現度優先。 | SVG 自身の配色（codex=白タイル＋グラデ / claude=クレイ） |

- **既定を mono にした意図**：ステータスカードは情報レイヤー。3 つのカードが並んだとき、
  バッジが各ブランド色で主張すると**状態（accent）よりロゴの色が目立ち**、視線が割れる。
  まず「形（ロゴ）」で出自を伝え、色は state 表現に譲る。だから既定はモノクロ統一。
- tint / brand は「色で出自を出したい」ときの選択肢として残す（playground で切替）。
- copilot は **GitHub Copilot のロボットヘッド**。公式に多色版が無いため brand でも単色（近似ダークで塗る）。

### 1.2 光学サイズ補正（`--badge-glyph-scale`）

枠（22×22）・SVG（20×20）・viewBox（24×24）は **3 ソースとも同寸**。にもかかわらず
Codex が大きく見えるのは、**ロゴが viewBox 内をどれだけ埋めるかの差**：

- Codex … 渦巻きが viewBox を **full-bleed**（端まで）。塗り面も広く視覚的に重い。
- Copilot … ロボ頭で上下に余白。
- Claude … 横は端まで・縦は中央の細い線画でインク量が少ない。

「箱は同じ・中身の占有率が違う」ので、**占有率を揃える光学補正**を入れる。

```
codex   : scale 0.84
copilot : scale 0.94
claude  : scale 1.0
```

> ルール：**バッジは枠サイズではなく "viewBox 内の占有率（＝見た目の大きさ）" を揃える。**
> 新しい source を足すときは、その SVG の埋まり方を見て scale を決める。

### 1.3 縦位置：タイトルの cap-height 中心に合わせる（`top: 6px`）

バッジの縦位置は、**タイトル 1 行目の cap-height（大文字の高さ）の中心**に光学中心を合わせる。
line-box の幾何中心ではない。

- タイトル：`14px` / line-height `18.2px`、カード上 padding `9px`。
- line-box 中心 ≈ `18.1px`。だが **cap-height 中心 ≈ `17.4px`**（cap top ~12.5 / cap 高 ~9.8）。
  タイトルはディセンダ余白を使わないので、line-box 中心で合わせると必ず**下に沈んで見える**。
- バッジ高 `22px`。cap 中心に合わせる → `top = 17.4 − 11 ≈ 6px`（バッジ中心 `17px`）。
  - 旧 `top: 8`（中心 19px）は cap 中心より ~1.6px 低く、「沈み」として知覚されていた。

> ルール：**アイコンとテキストの縦位置は、line-box ではなく cap-height の中心で揃える。**

### 1.4 丸形 ＋ close との morph（D4）

- バッジは `border-radius:50%` の丸いアイコンボタン。
- **カード hover で左上が crossfade して✖（削除）に変わる**（`.source-badge` opacity→0／`.status-card-close` opacity→1、
  同位置・同サイズ `22px`・`top:6px left:8px`）。常設の閉じるボタンは置かない。
- 右上はステータスアイコン（D3）に明け渡す。hover 中は出自が一時的に隠れる（タイトルの source 名で代替）。
- 本家 codex pets の「hover で左上に丸い✖」を踏襲。`docs/decisions.md` D4。

---

## 2. 影 ＝ elevation（接地影）

### 2.1 意図と光源

- ステータスカードの影は **背景からカードを少し持ち上げる接地影（elevation）**。装飾ではなく
  「白いカードが薄い背景の上にレイヤーとして乗っている」ことを示す。
- **光源は単一・真上**（横ずれ無し＝縦オフセットのみ）。フラット UI の定石で、全カードで一貫させ、
  カードごとに照明がブレないようにする。
  - （※ §3 の positional shadow を ON にすると、この「真上」を「デスクトップ上の固定光源」に拡張する。）

### 2.2 gap 連動ルール（汚れ回避）— 負の spread

**最重要の設計ルール。** 影が「カードの影」ではなく「汚れ・ムラ」に見える原因は診断できる：

> **落ち影の到達距離が、カード間の gap を越えると、隣り合うカードの影が連続した灰色の帯に
> 融合し、汚れに見える。**

- 旧 `y8 / blur22 / spread0` の到達は約 **30px**。gap は **6px**。到達 ≫ gap なので帯化＝汚れ。
- 単に blur を下げると浮きが死ぬ。だから **負の spread** で影の足元を内側に絞り、柔らかさを残したまま
  到達を gap 内に収める。

```
ルール: 影の実効到達 ≦ --card-gap
手段:   負の --card-shadow-spread で footprint を内側に縮める
        （footprint が小さくなる分、alpha を僅かに上げて存在感を補正）
```

### 2.3 既定値

```css
/* 0 を基準に、真上光源・gap 内に収まる接地影 */
--card-shadow-x: 0px;     /* positional shadow OFF 時。ON では動的（§3） */
--card-shadow-y: 6px;
--card-shadow-blur: 16px;
--card-shadow-spread: -8px;
--card-shadow-alpha: 0.2;
/* => box-shadow: 0 6px 16px -8px rgba(16,19,28,.2) + inset 0 0 0 1.2px rgba(29,36,51,.1) */
```

inset の極細枠（`1.2px`）は影とは別物（エッジ定義）で据え置き。

---

## 3. positional shadow（位置連動の影）＝ 身体性

### 3.1 思想

navi は**デスクトップ上を動くウィンドウ**。ならば「画面のどこにいるか」を影で感じられると、
ただの UI ではなく**その場所に在る物体**になる。デスクトップを 1 つのシーンと見なし、
**仮想光源を画面座標に固定**して、ウィンドウ位置から影を計算する。

### 3.2 モデル（相似三角形による水平射影）

- 光源は **上空・水平位置 `Light X`** に固定（既定は中央 `0.5`）。常に高所にあるので影は必ず下に落ちる。
- ウィンドウ（＝アバター）中心が光源より右にいれば影は右下へ、左なら左下へ倒れる。

```
ratio = clamp((windowCx − lightX) / (screenW/2), −1, +1)   // −1=左端 / 0=直下 / +1=右端
dist  = |ratio|                                             // 横距離（0=直下 / 1=端）

shadowX = ratio * leanStrength                             // 向き＋長さ（既定 leanStrength = 12px）
shadowY = base                                            // 据え置き（光源は常に上）
```

### 3.3 raking light（横距離で長く・柔らかく・薄く）

光源直下では光がほぼ垂直 → 影は短く・濃く・くっきり。横に離れると光が斜め（grazing）に当たる
→ 影が伸びて拡散し薄くなる。これを `dist` で駆動する：

```
blur  = baseBlur  + dist * blurGain          // 端ほど柔らかい（既定 blurGain = 10px）
alpha = baseAlpha * (1 − dist * fade)         // 端ほど薄い  （既定 fade = 0.35）
```

実測（中央 → 端、既定値）：`x 0→+10px` / `blur 16→24.3px` / `alpha 0.20→0.142`。

### 3.4 実装メモ

- `applyShadowVector()`（`playground.ts`）が `--card-shadow-x` / `--card-shadow-blur` /
  `--card-shadow-alpha` を**所有**し、アバターのドラッグ・コントロール変更のたびに再計算する。
  follow OFF のときだけ §2.3 の base がそのまま出る。
- **1 ウィンドウ＝1 ベクトルを共有**：スタック内のカードはすべて同じ影ベクトルを使う
  （画面距離に対してカード同士は十分近いため）。
- playground は stage 内座標で代用。**実機（Tauri）では各ペットが独立 OS ウィンドウ**なので、
  実際の**ウィンドウのスクリーン座標＋画面サイズ**を光源計算に渡す（将来実装。ここはモデル検証用）。

---

## 4. パラメータ早見表

| CSS 変数 / 概念 | 既定 | playground コントロール | 駆動 |
|---|---|---|---|
| `--badge-mono` | `#52525b` | Source colors（mono 時） | 静的 |
| `--badge-glyph-scale` | codex .84 / copilot .94 / claude 1.0 | （コード固定） | 静的 |
| バッジ `top` | `6px` | （コード固定・cap 中心整列） | 静的 |
| `--card-shadow-y` | `6px` | Shadow Y | 静的 |
| `--card-shadow-spread` | `-8px` | Shadow spread | 静的 |
| `--card-shadow-blur` | `16px`(base) | Shadow blur | §3 で動的 |
| `--card-shadow-alpha` | `0.20`(base) | Shadow opacity | §3 で動的 |
| `--card-shadow-x` | `0px` | （Light source 群が駆動） | §3 で動的 |
| positional: `Light X` | `0.5` | Light X | — |
| positional: `leanStrength` | `12px` | Lean strength | — |
| positional: `blurGain` | `10px` | Distance blur gain | — |
| positional: `fade` | `0.35` | Distance fade | — |

---

## 5. 設計ルールまとめ（迷ったらここへ戻る）

1. **ステータスカードは Live Activity**。静かで整った情報レイヤー。発話・トーストではない。
2. **バッジ既定はモノクロ**。色は state（accent）に譲り、出自はまず形で伝える。
3. **アイコン⇔テキストは cap-height 中心で揃える**（line-box 中心ではない）。
4. **バッジは占有率を揃える**（枠サイズではなく viewBox の埋まり方）。
5. **影の到達 ≦ gap**。越えたら汚れに見える。負の spread で内側に絞る。
6. **影は単一・真上光源の接地影**。positional ON でデスクトップ固定光源へ拡張。
7. **位置で身体性を出す**：横距離で倒れ・長さ・柔らかさ・濃さを変える（raking light）。

---

## 6. 今後

- playground で blurGain / fade / leanStrength の最終バランスをオーナーが確定（`issues/d9a2f7`）。
- 確定値を `@navi/ui` 既定／app シェルへ焼き込む（`c4b1e0` / `e1f5c3`）。
- positional shadow を**実機のウィンドウ・スクリーン座標**で駆動（Tauri 側で window position を供給）。
