import "@navi/ui";
import "./playground.css";
import { animate } from "motion";
import {
  type AgentState,
  type SourceId,
  type StatusCardData,
  agentStates,
  sourceConfig,
  stateLabels,
  highestPriorityState,
  isVisibleInAuto,
  createStatusCard,
  updateStatusCard,
} from "@navi/ui/navi";

// playground のセッションはステータスカードのデータ＋安定 id。
interface SessionData extends StatusCardData {
  id: string;
  source: SourceId;
}

// ─────────────────────────────────────────────────────────────────────────────
// playground = pet アバター × navi 固有 UI の統合デザインをチューニングするサンドボックス
// （issue d9a2f7）。スライダー等でパラメータを露出 → オーナーが良い値を読み取って指示
// → エージェントがコンポーネント既定値 / シェルへ焼き込む。
// ─────────────────────────────────────────────────────────────────────────────

type DisplayMode = "show" | "hide" | "auto";
type AccentStyle = "rail" | "tint" | "ring" | "shadow";

interface Params {
  petSize: number;
  cardWidth: number;
  cardPadX: number;
  cardPadY: number;
  cardRadius: number;
  cardGap: number;
  cardOffsetX: number;
  cardOffsetY: number;
  shadowY: number;
  shadowBlur: number;
  shadowAlpha: number;
  maxVisible: number;
  fps: number; // 0 = pet.json 既定
  displayMode: DisplayMode;
  accentStyle: AccentStyle; // state カラーの見せ方
  motion: boolean; // ステータスカードのマイクロモーション ON/OFF
  colors: Record<SourceId, string>;
}

const params: Params = {
  petSize: 128,
  cardWidth: 266,
  cardPadX: 10,
  cardPadY: 9,
  cardRadius: 12,
  cardGap: 6,
  cardOffsetX: 8,
  cardOffsetY: 8,
  shadowY: 8,
  shadowBlur: 22,
  shadowAlpha: 0.18,
  maxVisible: 3,
  fps: 0,
  displayMode: "show",
  accentStyle: "rail",
  motion: true,
  colors: {
    "claude-code": sourceConfig["claude-code"].color,
    codex: sourceConfig.codex.color,
    copilot: sourceConfig.copilot.color,
  },
};

// ── session 状態 ──

let seq = 0;
const sessions = new Map<string, SessionData>();
const cards = new Map<string, HTMLElement>();

// UI 名ラベル（glossary の正式名をステージ上に重ねるデバッグ表示）の ON/OFF。
let uiNames = false;

const sampleMessages: Record<SourceId, string> = {
  "claude-code": "Editing src/state.ts — adding the priority table",
  codex: "Waiting for approval to run `cargo test`",
  copilot: "Generating completions for navi-pet.ts",
};

function nextId(source: SourceId): string {
  seq += 1;
  return `${source}:${seq}`;
}

function makeSession(source: SourceId, state: AgentState): SessionData {
  return {
    id: nextId(source),
    source,
    state,
    label: `${sourceConfig[source].label} #${seq}`,
    message: sampleMessages[source],
    project: "agent-pets",
    cwd: "/home/user/agent-pets",
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// DOM 構築
// ─────────────────────────────────────────────────────────────────────────────

const root = document.querySelector<HTMLElement>("#playground");
if (!root) {
  throw new Error("Missing #playground root");
}

root.innerHTML = `
  <div class="pg-stage">
    <div class="navi-shell" aria-label="navi status">
      <div class="status-stack"></div>
      <div class="navi-pet-wrap">
        <button class="status-toggle" type="button" aria-label="Hide status cards" aria-pressed="true">
          <span class="toggle-chevron" aria-hidden="true"></span>
          <span class="session-count" aria-label="0 active agent sessions">0</span>
        </button>
        <navi-pet embedded pet="/pets/mio/pet.json" state="done"></navi-pet>
      </div>
    </div>
  </div>
  <aside class="pg-panel">
    <header class="pg-head">
      <h1>navi playground</h1>
      <p class="pg-sub">pet × navi UI 統合デザインのチューニング（Tauri 不要）</p>
    </header>
    <div class="pg-controls"></div>
    <section class="pg-readout">
      <div class="pg-readout-head">
        <h2>Current values</h2>
        <button type="button" class="pg-copy">Copy CSS</button>
      </div>
      <pre class="pg-readout-body"><code></code></pre>
    </section>
  </aside>
`;

const stage = root.querySelector<HTMLElement>(".pg-stage")!;
const shell = root.querySelector<HTMLElement>(".navi-shell")!;
const stackEl = root.querySelector<HTMLElement>(".status-stack")!;
const pet = root.querySelector<HTMLElement>("navi-pet")!;
const toggleBtn = root.querySelector<HTMLButtonElement>(".status-toggle")!;
const sessionCountEl = root.querySelector<HTMLElement>(".session-count")!;
const controlsEl = root.querySelector<HTMLElement>(".pg-controls")!;
const readoutCode = root.querySelector<HTMLElement>(".pg-readout-body code")!;
const copyBtn = root.querySelector<HTMLButtonElement>(".pg-copy")!;

// UI 名ラベルを描く透明オーバーレイ層（ステージに重ねる・操作は透過）。
const uiOverlay = document.createElement("div");
uiOverlay.className = "pg-ui-overlay";
stage.appendChild(uiOverlay);

const petWrap = root.querySelector<HTMLElement>(".navi-pet-wrap")!;

// 「アバターをドラッグできる」ことの案内（最初のドラッグで消える）。
const dragHint = document.createElement("div");
dragHint.className = "pg-drag-hint";
dragHint.textContent = "アバターをドラッグ → スタックが向きを変えて追従";
stage.appendChild(dragHint);

// ─────────────────────────────────────────────────────────────────────────────
// マイクロモーション（motion）＋ アバター位置に追従するレイアウト
//
// アバターはドラッグでステージ（＝デスクトップ）上のどこへでも動かせる。スタックは
// アバターの四象限（左右 × 上下）に応じて、はみ出さない側へ展開する。象限をまたいだ
// 瞬間や、カードの追加・削除時は FLIP で滑らかに整列し直す。`status-card.ts` は
// 「props in / DOM out」のダム部品のままなので、これらの演出は consumer 側に置く。
// ─────────────────────────────────────────────────────────────────────────────

// アバター左上のステージ内座標（px）。drag が書き込み、CSS 変数 --pet-x/-y に反映。
let petX = 0;
let petY = 0;
const EDGE = 12; // ステージ端からの最小すきま

// reduced-motion を尊重。OS 設定で無効、または Motion トグル OFF なら一切動かさない。
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
function motionOn(): boolean {
  return params.motion && !reduceMotion.matches;
}

const ENTER_SPRING = { type: "spring", stiffness: 560, damping: 30, mass: 0.9 } as const;
const FLIP_SPRING = { type: "spring", stiffness: 650, damping: 38, mass: 0.9 } as const;

type Anchor = { h: "left" | "right"; v: "top" | "bottom" };

function petBox(): { w: number; h: number } {
  const w = params.petSize;
  return { w, h: (w * 208) / 192 };
}

// pet 中心がステージのどの象限にあるか → スタックの展開方向。
function currentAnchor(): Anchor {
  const stageRect = stage.getBoundingClientRect();
  const { w, h } = petBox();
  const cx = petX + w / 2;
  const cy = petY + h / 2;
  return {
    h: cx < stageRect.width / 2 ? "left" : "right",
    v: cy < stageRect.height / 2 ? "top" : "bottom",
  };
}

function clampPet(): void {
  const stageRect = stage.getBoundingClientRect();
  const { w, h } = petBox();
  petX = Math.max(EDGE, Math.min(petX, stageRect.width - w - EDGE));
  petY = Math.max(EDGE, Math.min(petY, stageRect.height - h - EDGE));
}

function applyPetPosition(): void {
  shell.style.setProperty("--pet-x", `${petX}px`);
  shell.style.setProperty("--pet-y", `${petY}px`);
}

// 現在見えているカードの位置を記録（FLIP の First）。
function captureCardRects(): Map<string, DOMRect> {
  const m = new Map<string, DOMRect>();
  for (const [id, el] of cards) m.set(id, el.getBoundingClientRect());
  return m;
}

// 記録位置から現在位置への差分を打ち消すように動かす（FLIP の Invert→Play）。
// skip の id（新規カードなど）は対象外。
function flipReflow(before: Map<string, DOMRect>, skip?: Set<string>): void {
  if (!motionOn()) return;
  for (const [id, el] of cards) {
    if (skip?.has(id)) continue;
    const prev = before.get(id);
    if (!prev) continue;
    const now = el.getBoundingClientRect();
    const dx = prev.left - now.left;
    const dy = prev.top - now.top;
    if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
    animate(el, { x: [dx, 0], y: [dy, 0] }, FLIP_SPRING);
  }
}

// 新規カードの登場（pet 側からせり出してスプリングで着地）。
function animateCardIn(el: HTMLElement): void {
  if (!motionOn()) return;
  const fromY = currentAnchor().v === "bottom" ? 10 : -10;
  animate(el, { opacity: [0, 1], y: [fromY, 0], scale: [0.94, 1] }, ENTER_SPRING);
}

// カードの退場（その場でフェード＆縮小）。完了を待てるよう Promise を返す。
async function animateCardOut(el: HTMLElement): Promise<void> {
  if (!motionOn()) return;
  const toY = currentAnchor().v === "bottom" ? 8 : -8;
  await animate(
    el,
    { opacity: [1, 0], y: [0, toY], scale: [1, 0.92] },
    { duration: 0.16, ease: "easeIn" },
  ).finished;
}

// アンカー（象限）クラスを反映。変化時は FLIP で整列し直す。
function applyAnchor(): void {
  const a = currentAnchor();
  const hClass = `anchor-h-${a.h}`;
  const vClass = `anchor-v-${a.v}`;
  const changed =
    !shell.classList.contains(hClass) || !shell.classList.contains(vClass);
  const before = changed ? captureCardRects() : null;
  shell.classList.remove(
    "anchor-h-left",
    "anchor-h-right",
    "anchor-v-top",
    "anchor-v-bottom",
  );
  shell.classList.add(hClass, vClass);
  if (before) flipReflow(before);
}

// 右下（従来の見え方）に初期配置。stage 実寸が要るので mount 後に呼ぶ。
function placePetInitial(): void {
  const stageRect = stage.getBoundingClientRect();
  const { w, h } = petBox();
  const pad = 28; // --stage-pad 相当
  petX = stageRect.width - w - pad;
  petY = stageRect.height - h - pad;
  clampPet();
  applyPetPosition();
  applyAnchor();
}

// pet サイズ変更・ウィンドウリサイズ後に内側へクランプし直す。
function reclampPet(): void {
  clampPet();
  applyPetPosition();
  applyAnchor();
  refreshUiNames();
}

// ── pet drag（ステージ＝デスクトップ上を移動） ──
// 埋め込み navi-pet は自前で pointer capture するが embedded では自分は動かない。
// pointerdown は wrap まで伝播するのでここで拾い、drag 中は window で move/up を追う
// （navi-pet と同方式・capture 中も window はイベントを受ける）。
let dragging = false;
let dragOriginX = 0;
let dragOriginY = 0;
let petOriginX = 0;
let petOriginY = 0;

petWrap.addEventListener("pointerdown", (e) => {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest(".status-toggle")) return; // トグルは別操作
  dragging = true;
  dragOriginX = e.clientX;
  dragOriginY = e.clientY;
  petOriginX = petX;
  petOriginY = petY;
  petWrap.classList.add("dragging");
  dragHint.remove();
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragEnd);
  window.addEventListener("pointercancel", onDragEnd);
});

function onDragMove(e: PointerEvent): void {
  if (!dragging) return;
  petX = petOriginX + (e.clientX - dragOriginX);
  petY = petOriginY + (e.clientY - dragOriginY);
  clampPet();
  applyPetPosition();
  applyAnchor();
  refreshUiNames();
}

function onDragEnd(): void {
  dragging = false;
  petWrap.classList.remove("dragging");
  window.removeEventListener("pointermove", onDragMove);
  window.removeEventListener("pointerup", onDragEnd);
  window.removeEventListener("pointercancel", onDragEnd);
}

// ─────────────────────────────────────────────────────────────────────────────
// コントロール用の小さなビルダー群
// ─────────────────────────────────────────────────────────────────────────────

function group(title: string): HTMLElement {
  const section = document.createElement("section");
  section.className = "pg-group";
  section.innerHTML = `<h2>${title}</h2><div class="pg-group-body"></div>`;
  controlsEl.appendChild(section);
  return section.querySelector<HTMLElement>(".pg-group-body")!;
}

interface SliderOpts {
  label: string;
  min: number;
  max: number;
  step?: number;
  value: number;
  unit?: string;
  onInput: (v: number) => void;
}

function slider(parent: HTMLElement, o: SliderOpts): void {
  const row = document.createElement("label");
  row.className = "pg-row pg-row-slider";
  row.innerHTML = `
    <span class="pg-label">${o.label}</span>
    <input type="range" min="${o.min}" max="${o.max}" step="${o.step ?? 1}" value="${o.value}" />
    <output>${o.value}${o.unit ?? ""}</output>
  `;
  const input = row.querySelector<HTMLInputElement>("input")!;
  const out = row.querySelector<HTMLOutputElement>("output")!;
  input.addEventListener("input", () => {
    const v = Number(input.value);
    out.textContent = `${v}${o.unit ?? ""}`;
    o.onInput(v);
  });
  parent.appendChild(row);
}

function toggle(
  parent: HTMLElement,
  label: string,
  value: boolean,
  onChange: (v: boolean) => void,
): void {
  const row = document.createElement("label");
  row.className = "pg-row pg-row-toggle";
  row.innerHTML = `<span class="pg-label">${label}</span><input type="checkbox" ${value ? "checked" : ""} />`;
  const input = row.querySelector<HTMLInputElement>("input")!;
  input.addEventListener("change", () => onChange(input.checked));
  parent.appendChild(row);
}

function color(
  parent: HTMLElement,
  label: string,
  value: string,
  onInput: (v: string) => void,
): void {
  const row = document.createElement("label");
  row.className = "pg-row pg-row-color";
  row.innerHTML = `<span class="pg-label">${label}</span><input type="color" value="${value}" /><code>${value}</code>`;
  const input = row.querySelector<HTMLInputElement>("input")!;
  const code = row.querySelector<HTMLElement>("code")!;
  input.addEventListener("input", () => {
    code.textContent = input.value;
    onInput(input.value);
  });
  parent.appendChild(row);
}

function segmented<T extends string>(
  parent: HTMLElement,
  label: string,
  options: T[],
  value: T,
  onPick: (v: T) => void,
): { set: (v: T) => void } {
  const row = document.createElement("div");
  row.className = "pg-row pg-row-segmented";
  row.innerHTML = `<span class="pg-label">${label}</span><div class="pg-seg"></div>`;
  const seg = row.querySelector<HTMLElement>(".pg-seg")!;
  const btns = new Map<T, HTMLButtonElement>();
  const set = (v: T) => {
    for (const [k, b] of btns) b.setAttribute("aria-pressed", String(k === v));
  };
  for (const opt of options) {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = opt;
    b.addEventListener("click", () => {
      set(opt);
      onPick(opt);
    });
    btns.set(opt, b);
    seg.appendChild(b);
  }
  set(value);
  parent.appendChild(row);
  return { set };
}

// ─────────────────────────────────────────────────────────────────────────────
// 反映：params → CSS 変数 / pet / ステータスカード可視性 / readout
// ─────────────────────────────────────────────────────────────────────────────

function getHighestState(): AgentState {
  return highestPriorityState(Array.from(sessions.values(), (s) => s.state));
}

function cardsVisible(): boolean {
  if (params.displayMode === "show") return true;
  if (params.displayMode === "hide") return false;
  return isVisibleInAuto(getHighestState());
}

function apply(): void {
  const s = shell.style;
  s.setProperty("--pet-size", `${params.petSize}px`);
  pet.style.setProperty("--navi-pet-size", `${params.petSize}px`);
  s.setProperty("--card-width", `${params.cardWidth}px`);
  s.setProperty("--card-pad-x", `${params.cardPadX}px`);
  s.setProperty("--card-pad-y", `${params.cardPadY}px`);
  s.setProperty("--card-radius", `${params.cardRadius}px`);
  s.setProperty("--card-gap", `${params.cardGap}px`);
  s.setProperty("--card-offset-x", `${params.cardOffsetX}px`);
  s.setProperty("--card-offset-y", `${params.cardOffsetY}px`);
  s.setProperty("--card-shadow-y", `${params.shadowY}px`);
  s.setProperty("--card-shadow-blur", `${params.shadowBlur}px`);
  s.setProperty("--card-shadow-alpha", String(params.shadowAlpha));
  s.setProperty("--card-max-visible", String(params.maxVisible));
  s.setProperty("--src-claude-code", params.colors["claude-code"]);
  s.setProperty("--src-codex", params.colors.codex);
  s.setProperty("--src-copilot", params.colors.copilot);

  shell.dataset.accent = params.accentStyle;

  if (params.fps > 0) pet.setAttribute("fps", String(params.fps));
  else pet.removeAttribute("fps");

  pet.setAttribute("state", getHighestState());

  const visible = cardsVisible();
  shell.classList.toggle("status-hidden", !visible);
  toggleBtn.setAttribute("aria-pressed", String(visible));
  toggleBtn.setAttribute(
    "aria-label",
    visible ? "Hide status cards" : "Show status cards",
  );

  const count = sessions.size;
  sessionCountEl.textContent = String(count);
  sessionCountEl.setAttribute(
    "aria-label",
    `${count} active agent session${count !== 1 ? "s" : ""}`,
  );

  refreshReadout();
  refreshUiNames();
}

function refreshReadout(): void {
  const lines = [
    "/* navi UI — tuned in playground */",
    ".pet-shell {",
    `  --pet-size: ${params.petSize}px;`,
    `  --card-width: ${params.cardWidth}px;`,
    `  --card-pad: ${params.cardPadY}px ${params.cardPadX}px;`,
    `  --card-radius: ${params.cardRadius}px;`,
    `  --card-gap: ${params.cardGap}px;`,
    `  --card-offset-x: ${params.cardOffsetX}px;`,
    `  --card-offset-y: ${params.cardOffsetY}px;`,
    `  --card-shadow: 0 ${params.shadowY}px ${params.shadowBlur}px rgba(16, 19, 28, ${params.shadowAlpha});`,
    `  --card-max-visible: ${params.maxVisible};`,
    `  --src-claude-code: ${params.colors["claude-code"]};`,
    `  --src-codex: ${params.colors.codex};`,
    `  --src-copilot: ${params.colors.copilot};`,
    "}",
    "",
    `/* state-accent: ${params.accentStyle} · pet fps: ${params.fps > 0 ? params.fps : "pet.json default"} · display-mode: ${params.displayMode} · card-motion: ${params.motion ? "on" : "off"} */`,
  ];
  readoutCode.textContent = lines.join("\n");
}

// glossary（docs/glossary.md）の正式名をステージ上の各部品へ重ねるデバッグ表示。
// 名前は glossary §1 と一致させること。
function refreshUiNames(): void {
  uiOverlay.replaceChildren();
  for (const el of stage.querySelectorAll(".ui-named")) el.classList.remove("ui-named");
  if (!uiNames) return;

  const stageRect = stage.getBoundingClientRect();
  const placed: { x: number; y: number; w: number; h: number }[] = [];
  const overlaps = (a: { x: number; y: number; w: number; h: number }): boolean =>
    placed.some(
      (b) =>
        !(a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y),
    );
  const place = (
    el: Element | null | undefined,
    name: string,
    side: "left" | "right" | "top" | "bottom",
  ): void => {
    if (!el) return;
    const target = el as HTMLElement;
    if (getComputedStyle(target).display === "none") return;
    const r = target.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return;

    const pill = document.createElement("div");
    pill.className = "ui-name-pill";
    pill.textContent = name;
    uiOverlay.appendChild(pill);

    const pw = pill.offsetWidth;
    const ph = pill.offsetHeight;
    const ex = r.left - stageRect.left;
    const ey = r.top - stageRect.top;
    let x = 0;
    let y = 0;
    if (side === "left") {
      x = ex - pw - 6;
      y = ey + r.height / 2 - ph / 2;
    } else if (side === "right") {
      x = ex + r.width + 6;
      y = ey + r.height / 2 - ph / 2;
    } else if (side === "top") {
      x = ex + r.width / 2 - pw / 2;
      y = ey - ph - 6;
    } else {
      x = ex + r.width / 2 - pw / 2;
      y = ey + r.height + 6;
    }
    // ステージ内へクランプ（はみ出して切れないように）
    x = Math.max(2, Math.min(x, stageRect.width - pw - 2));
    y = Math.max(2, Math.min(y, stageRect.height - ph - 2));
    // 既存ラベルと重なるなら縦にずらす（近接部品＝表示トグル等の衝突回避）
    const step = ph + 3;
    for (const off of [0, step, -step, 2 * step, -2 * step, 3 * step, -3 * step]) {
      const cand = Math.max(2, Math.min(y + off, stageRect.height - ph - 2));
      y = cand;
      if (!overlaps({ x, y, w: pw, h: ph })) break;
    }
    placed.push({ x, y, w: pw, h: ph });
    pill.style.left = `${x}px`;
    pill.style.top = `${y}px`;
    target.classList.add("ui-named");
  };

  // 常に見える部品
  place(pet, "アバター", "left");
  place(toggleBtn, "表示トグル", "right");
  if (getComputedStyle(sessionCountEl).display !== "none") {
    place(sessionCountEl, "セッションカウント", "right");
  }

  // スタックが見えているときだけ、その内側の部品を案内
  if (!shell.classList.contains("status-hidden")) {
    place(stackEl, "ステータススタック", "top");
    const cards = stackEl.querySelectorAll(".status-card");
    const lastCard = cards[cards.length - 1] as HTMLElement | undefined;
    place(lastCard, "ステータスカード", "left");
    place(lastCard?.querySelector(".source-badge"), "ソースバッジ", "left");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// ステータスカード ↔ session の同期
// ─────────────────────────────────────────────────────────────────────────────

let editorBody: HTMLElement | null = null;

function renderStatusCards(): void {
  // 整列アニメ（FLIP）のため、変更前のカード位置を記録。
  const before = captureCardRects();
  const fresh = new Set<string>();

  // 既存セッション分を upsert
  for (const session of sessions.values()) {
    let el = cards.get(session.id);
    if (!el) {
      el = createStatusCard(session.id, session, { onClose: removeSession });
      cards.set(session.id, el);
      stackEl.appendChild(el);
      fresh.add(session.id);
    } else {
      updateStatusCard(el, session);
    }
  }
  // 消えたセッションの card を除去（アニメ無し経路。退場演出は removeSession 側）
  for (const [id, el] of cards) {
    if (!sessions.has(id)) {
      el.remove();
      cards.delete(id);
    }
  }

  // 既存カードは新レイアウトへスプリングで詰め、新規カードは登場アニメ。
  flipReflow(before, fresh);
  for (const id of fresh) {
    const el = cards.get(id);
    if (el) animateCardIn(el);
  }
}

function renderEditors(): void {
  if (!editorBody) return;
  editorBody.replaceChildren();

  if (sessions.size === 0) {
    const empty = document.createElement("p");
    empty.className = "pg-empty";
    empty.textContent = "No sessions. Add one to see the card stack.";
    editorBody.appendChild(empty);
    return;
  }

  for (const session of sessions.values()) {
    editorBody.appendChild(editorRow(session));
  }
}

function editorRow(session: SessionData): HTMLElement {
  const row = document.createElement("div");
  row.className = "pg-session";
  row.dataset.state = session.state;

  const sourceOpts = (Object.keys(sourceConfig) as SourceId[])
    .map((s) => `<option value="${s}" ${s === session.source ? "selected" : ""}>${sourceConfig[s].label}</option>`)
    .join("");
  const stateOpts = agentStates
    .map((st) => `<option value="${st}" ${st === session.state ? "selected" : ""}>${stateLabels[st]}</option>`)
    .join("");

  row.innerHTML = `
    <div class="pg-session-head">
      <select class="pg-session-source" aria-label="Source">${sourceOpts}</select>
      <select class="pg-session-state" aria-label="State">${stateOpts}</select>
      <button type="button" class="pg-session-del" aria-label="Remove session">×</button>
    </div>
    <input class="pg-session-title" type="text" value="${escapeAttr(session.label)}" aria-label="Title" />
    <input class="pg-session-msg" type="text" value="${escapeAttr(session.message ?? "")}" placeholder="message (blank → state label)" aria-label="Message" />
    <input class="pg-session-proj" type="text" value="${escapeAttr(session.project ?? "")}" placeholder="project name" aria-label="Project" />
  `;

  const sourceSel = row.querySelector<HTMLSelectElement>(".pg-session-source")!;
  const stateSel = row.querySelector<HTMLSelectElement>(".pg-session-state")!;
  const title = row.querySelector<HTMLInputElement>(".pg-session-title")!;
  const msg = row.querySelector<HTMLInputElement>(".pg-session-msg")!;
  const proj = row.querySelector<HTMLInputElement>(".pg-session-proj")!;

  sourceSel.addEventListener("change", () => {
    session.source = sourceSel.value as SourceId;
    syncSession(session);
  });
  stateSel.addEventListener("change", () => {
    session.state = stateSel.value as AgentState;
    row.dataset.state = session.state;
    syncSession(session);
  });
  title.addEventListener("input", () => {
    session.label = title.value;
    syncSession(session);
  });
  msg.addEventListener("input", () => {
    session.message = msg.value;
    syncSession(session);
  });
  proj.addEventListener("input", () => {
    session.project = proj.value;
    syncSession(session);
  });
  row
    .querySelector<HTMLButtonElement>(".pg-session-del")!
    .addEventListener("click", () => removeSession(session.id));

  return row;
}

/** session 1件の変更を card に反映（エディタは再構築しない＝フォーカス維持）。 */
function syncSession(session: SessionData): void {
  const el = cards.get(session.id);
  if (el) updateStatusCard(el, session);
  apply();
}

function addSession(source: SourceId, state: AgentState = "running"): void {
  const session = makeSession(source, state);
  sessions.set(session.id, session);
  renderStatusCards();
  renderEditors();
  apply();
}

function removeSession(id: string): void {
  const el = cards.get(id);
  if (!el || !motionOn()) {
    sessions.delete(id);
    renderStatusCards();
    renderEditors();
    apply();
    return;
  }
  // 退場: その場でフェード → DOM 除去 → 残りを FLIP で詰める。
  void (async () => {
    await animateCardOut(el);
    const before = captureCardRects(); // el を含む（まだ index に在る）
    el.remove();
    cards.delete(id);
    sessions.delete(id);
    flipReflow(before, new Set([id]));
    renderEditors();
    apply();
  })();
}

function escapeAttr(v: string): string {
  return v.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}

// ─────────────────────────────────────────────────────────────────────────────
// パネル構築
// ─────────────────────────────────────────────────────────────────────────────

// Stack グループで代入、shell トグルから参照する。
let displayModeControl!: { set: (v: DisplayMode) => void };

// Sessions
{
  const body = group("Sessions");
  const actions = document.createElement("div");
  actions.className = "pg-session-actions";
  const sources = Object.keys(sourceConfig) as SourceId[];
  for (const src of sources) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "pg-add";
    b.textContent = `+ ${sourceConfig[src].label}`;
    b.addEventListener("click", () => addSession(src));
    actions.appendChild(b);
  }
  body.appendChild(actions);

  const presets = document.createElement("div");
  presets.className = "pg-session-actions";
  const longBtn = button("Long message", () => {
    addSession("claude-code", "thinking");
    const last = Array.from(sessions.values()).at(-1)!;
    last.message =
      "Refactoring the highest-priority state resolver so that multiple concurrent agent sessions collapse into a single pet animation without dropping the per-session status cards — this line is intentionally very long to test overflow.";
    last.project = "a-rather-long-monorepo-project-name-for-overflow";
    last.label = "Claude Code — long running task with a long title";
    syncSession(last);
    renderEditors();
  });
  const fillBtn = button("Fill ×5", () => {
    const order: [SourceId, AgentState][] = [
      ["claude-code", "running"],
      ["codex", "waiting_approval"],
      ["copilot", "editing"],
      ["claude-code", "error"],
      ["codex", "thinking"],
    ];
    for (const [src, st] of order) addSession(src, st);
  });
  const clearBtn = button("Clear", () => {
    if (!motionOn() || cards.size === 0) {
      sessions.clear();
      renderStatusCards();
      renderEditors();
      apply();
      return;
    }
    // 全カードをフェードアウトしてから除去。
    const els = Array.from(cards.values());
    sessions.clear();
    cards.clear();
    renderEditors();
    apply();
    void Promise.all(els.map((el) => animateCardOut(el))).then(() => {
      for (const el of els) el.remove();
    });
  });
  presets.append(longBtn, fillBtn, clearBtn);
  body.appendChild(presets);

  const list = document.createElement("div");
  list.className = "pg-session-list";
  body.appendChild(list);
  editorBody = list;
}

function button(label: string, onClick: () => void): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "pg-add";
  b.textContent = label;
  b.addEventListener("click", onClick);
  return b;
}

// Pet
{
  const body = group("Pet");
  slider(body, {
    label: "Size",
    min: 64,
    max: 256,
    value: params.petSize,
    unit: "px",
    onInput: (v) => {
      params.petSize = v;
      reclampPet(); // 大きくしたとき端からはみ出さないよう内側へ寄せる
      apply();
    },
  });
  slider(body, {
    label: "Anim fps (0 = default)",
    min: 0,
    max: 16,
    value: params.fps,
    onInput: (v) => {
      params.fps = v;
      apply();
    },
  });
}

// Stack
{
  const body = group("Stack");
  displayModeControl = segmented<DisplayMode>(
    body,
    "Mode",
    ["show", "hide", "auto"],
    params.displayMode,
    (v) => {
      params.displayMode = v;
      apply();
    },
  );
  slider(body, {
    label: "Max visible (then scroll)",
    min: 1,
    max: 6,
    value: params.maxVisible,
    onInput: (v) => {
      params.maxVisible = v;
      apply();
    },
  });
  slider(body, {
    label: "Stack gap",
    min: 0,
    max: 24,
    value: params.cardGap,
    unit: "px",
    onInput: (v) => {
      params.cardGap = v;
      apply();
    },
  });
}

// Card
{
  const body = group("Card");
  segmented<AccentStyle>(
    body,
    "State accent",
    ["rail", "tint", "ring", "shadow"],
    params.accentStyle,
    (v) => {
      params.accentStyle = v;
      apply();
    },
  );
  const defs: SliderOpts[] = [
    { label: "Max width", min: 180, max: 360, value: params.cardWidth, unit: "px", onInput: (v) => (params.cardWidth = v) },
    { label: "Padding X", min: 4, max: 24, value: params.cardPadX, unit: "px", onInput: (v) => (params.cardPadX = v) },
    { label: "Padding Y", min: 4, max: 24, value: params.cardPadY, unit: "px", onInput: (v) => (params.cardPadY = v) },
    { label: "Corner radius", min: 0, max: 24, value: params.cardRadius, unit: "px", onInput: (v) => (params.cardRadius = v) },
    { label: "Offset X (from pet)", min: 0, max: 60, value: params.cardOffsetX, unit: "px", onInput: (v) => (params.cardOffsetX = v) },
    { label: "Offset Y (above pet)", min: 0, max: 60, value: params.cardOffsetY, unit: "px", onInput: (v) => (params.cardOffsetY = v) },
    { label: "Shadow Y", min: 0, max: 24, value: params.shadowY, unit: "px", onInput: (v) => (params.shadowY = v) },
    { label: "Shadow blur", min: 0, max: 48, value: params.shadowBlur, unit: "px", onInput: (v) => (params.shadowBlur = v) },
  ];
  for (const d of defs) {
    const orig = d.onInput;
    slider(body, {
      ...d,
      onInput: (v) => {
        orig(v);
        apply();
      },
    });
  }
  slider(body, {
    label: "Shadow opacity",
    min: 0,
    max: 0.5,
    step: 0.01,
    value: params.shadowAlpha,
    onInput: (v) => {
      params.shadowAlpha = v;
      apply();
    },
  });
}

// Source colors
{
  const body = group("Source colors");
  color(body, "Claude Code", params.colors["claude-code"], (v) => {
    params.colors["claude-code"] = v;
    apply();
  });
  color(body, "Codex", params.colors.codex, (v) => {
    params.colors.codex = v;
    apply();
  });
  color(body, "Copilot", params.colors.copilot, (v) => {
    params.colors.copilot = v;
    apply();
  });
}

// Motion
{
  const body = group("Motion");
  toggle(body, "Card micro-motion", params.motion, (v) => {
    params.motion = v;
    apply();
  });
  const note = document.createElement("p");
  note.className = "pg-note";
  note.textContent =
    "OS の reduced-motion を尊重（有効時は自動で無効）。アバターはドラッグで移動でき、スタックは象限に合わせて再配置される。";
  body.appendChild(note);
}

// Debug
{
  const body = group("Debug");
  toggle(body, "UI 名ラベル (show UI names)", uiNames, (v) => {
    uiNames = v;
    refreshUiNames();
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// shell 内のトグル / コピー
// ─────────────────────────────────────────────────────────────────────────────

toggleBtn.addEventListener("click", () => {
  params.displayMode = cardsVisible() ? "hide" : "show";
  displayModeControl.set(params.displayMode);
  apply();
});

copyBtn.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(readoutCode.textContent ?? "");
    copyBtn.textContent = "Copied!";
    setTimeout(() => (copyBtn.textContent = "Copy CSS"), 1200);
  } catch {
    copyBtn.textContent = "Copy failed";
    setTimeout(() => (copyBtn.textContent = "Copy CSS"), 1200);
  }
});

// pet が毎フレーム書き込む current-animation をデバッグ表示（stage 隅）
const animBadge = document.createElement("div");
animBadge.className = "pg-anim-badge";
stage.appendChild(animBadge);
const observer = new MutationObserver(() => {
  animBadge.textContent = `${getHighestState()} → ${pet.getAttribute("current-animation") ?? "–"}`;
});
observer.observe(pet, { attributes: true, attributeFilter: ["current-animation"] });

// UI 名ラベルはステージ上の実位置に追従させる（スクロール / リサイズで再計算）。
stackEl.addEventListener("scroll", refreshUiNames);
window.addEventListener("resize", refreshUiNames);

// ─────────────────────────────────────────────────────────────────────────────
// 初期セッション
// ─────────────────────────────────────────────────────────────────────────────

placePetInitial(); // 右下（従来の見え方）に置く。anchor-* クラスもここで付く。
window.addEventListener("resize", reclampPet);

addSession("claude-code", "running");
addSession("codex", "waiting_approval");
apply();
