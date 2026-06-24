import { createPetCore } from "./pet-core";
import type { CoreAnimation, PetCore } from "./pet-core";

// codex-pet-web の <codex-pet>（src/codex-pet.ts）を navi 用に移植。
// 変更点:
//  - MoonBit 依存を排除し純TSの pet-core を使用
//  - navi の AgentState を `state` 属性で受け、内部でアニメ名へ変換
//  - playground 用に `embedded` 属性（position:fixed を解除）を追加

type PetManifest = {
  id?: string;
  displayName?: string;
  description?: string;
  spritesheetPath: string;
  columns?: number;
  rows?: number;
  frameWidth?: number;
  frameHeight?: number;
  animations?: Record<string, AnimationConfig>;
};

type AnimationConfig = {
  row?: number;
  frames?: number[];
  fps?: number;
};

type RuntimePetConfig = {
  name: string;
  src: string;
  columns: number;
  rows: number;
  frameWidth?: number;
  frameHeight?: number;
  animations: Record<string, Required<AnimationConfig>>;
};

export type NaviPetSpeechOptions = {
  duration?: number;
  position?: "top" | "bottom" | "left" | "right";
};

// navi の AgentState -> codex-pet-web 規約のアニメ名。
// done=idle / running=running / waiting_approval=waving / error=failed / thinking=review。
// editing は暫定で thinking と共用（review）。
const stateToAnimation: Record<string, string> = {
  done: "idle",
  running: "running",
  editing: "review",
  thinking: "review",
  waiting_approval: "waving",
  error: "failed",
};

const defaultAnimations: Record<string, Required<AnimationConfig>> = {
  idle: { row: 0, frames: [0, 1, 2, 3, 4, 5], fps: 8 },
  "running-right": { row: 1, frames: [0, 1, 2, 3, 4, 5, 6, 7], fps: 10 },
  "running-left": { row: 2, frames: [0, 1, 2, 3, 4, 5, 6, 7], fps: 10 },
  waving: { row: 3, frames: [0, 1, 2, 3], fps: 8 },
  jumping: { row: 4, frames: [0, 1, 2, 3, 4], fps: 9 },
  failed: { row: 5, frames: [0, 1, 2, 3, 4, 5, 6, 7], fps: 6 },
  waiting: { row: 6, frames: [0, 1, 2, 3, 4, 5], fps: 6 },
  running: { row: 7, frames: [0, 1, 2, 3, 4, 5], fps: 8 },
  review: { row: 8, frames: [0, 1, 2, 3, 4, 5], fps: 6 },
  walk: { row: 7, frames: [0, 1, 2, 3, 4, 5], fps: 8 },
  play: { row: 3, frames: [0, 1, 2, 3], fps: 8 },
  jump: { row: 4, frames: [0, 1, 2, 3, 4], fps: 9 },
  sleep: { row: 6, frames: [0, 1, 2, 3, 4, 5], fps: 6 },
};

const template = document.createElement("template");

template.innerHTML = `
  <style>
    :host {
      --navi-pet-size: 144px;
      --navi-pet-accent: #6c8cff;
      --navi-pet-shadow: 0 12px 24px rgb(0 0 0 / 22%);
      display: block;
      inline-size: var(--navi-pet-size);
      aspect-ratio: var(--navi-pet-frame-ratio, 192 / 208);
      position: fixed;
      right: 24px;
      bottom: 24px;
      z-index: 2147483000;
      touch-action: none;
      cursor: grab;
      user-select: none;
    }

    :host([embedded]) {
      position: static;
      inset: auto;
      right: auto;
      bottom: auto;
    }

    :host([dragging]) {
      cursor: grabbing;
    }

    :host([resizing]) {
      cursor: nwse-resize;
    }

    .resize-handle {
      position: absolute;
      inline-size: 18px;
      block-size: 18px;
      right: -2px;
      bottom: -2px;
      border: 0;
      border-right: 4px solid color-mix(in srgb, var(--navi-pet-accent), black 30%);
      border-bottom: 4px solid color-mix(in srgb, var(--navi-pet-accent), black 30%);
      padding: 0;
      background: transparent;
      cursor: nwse-resize;
      filter: drop-shadow(0 1px 1px rgb(255 255 255 / 65%));
      opacity: 0;
      pointer-events: none;
    }

    :host([resize-handle]) .resize-handle,
    :host([resizing]) .resize-handle {
      opacity: 1;
      pointer-events: auto;
    }

    .pet {
      inline-size: 100%;
      block-size: 100%;
      background-image: var(--navi-pet-spritesheet);
      background-repeat: no-repeat;
      background-position: var(--navi-pet-frame-x, 0%) var(--navi-pet-frame-y, 0%);
      background-size: calc(var(--navi-pet-columns, 8) * 100%) calc(var(--navi-pet-rows, 9) * 100%);
      filter: drop-shadow(var(--navi-pet-shadow));
      image-rendering: pixelated;
      opacity: 0;
      transition: opacity 140ms ease;
    }

    :host([ready]) .pet {
      opacity: 1;
    }

    .speech {
      position: absolute;
      inline-size: max-content;
      max-inline-size: min(240px, 70vw);
      padding: 8px 10px;
      border: 1px solid rgb(0 0 0 / 14%);
      border-radius: 8px;
      background: rgb(255 255 255 / 96%);
      box-shadow: 0 8px 22px rgb(0 0 0 / 18%);
      color: #17201d;
      font: 500 13px/1.35 system-ui, sans-serif;
      letter-spacing: 0;
      overflow-wrap: anywhere;
      right: 0;
      bottom: calc(100% + 10px);
      opacity: 0;
      pointer-events: none;
      transform: translateY(4px);
      transition: opacity 140ms ease, transform 140ms ease;
    }

    :host([speech-open]) .speech {
      opacity: 1;
      transform: translateY(0);
    }
  </style>
  <div class="pet" part="pet" role="img" aria-label="navi pet"></div>
  <div class="speech" part="speech" role="status" aria-live="polite"></div>
  <button class="resize-handle" part="resize-handle" type="button" aria-label="Resize navi pet"></button>
`;

export class NaviPetElement extends HTMLElement {
  static observedAttributes = [
    "animation",
    "state",
    "fps",
    "pet",
    "position",
    "speech",
    "speech-duration",
    "src",
  ];

  readonly #root: ShadowRoot;
  #animationFrame = 0;
  #config: RuntimePetConfig | undefined;
  #core: PetCore = createPetCore();
  #loadToken = 0;
  #pointerId: number | undefined;
  #pet: HTMLElement;
  #resizeHandle: HTMLElement;
  #speech: HTMLElement;
  #speechTimer = 0;
  #dragDirectionX = 1;
  #lastPointerX = 0;
  #resizing = false;
  #startLeft = 0;
  #startPointerX = 0;
  #startPointerY = 0;
  #startSize = 0;
  #startTop = 0;

  constructor() {
    super();
    this.#root = this.attachShadow({ mode: "open" });
    this.#root.append(template.content.cloneNode(true));

    const pet = this.#root.querySelector<HTMLElement>(".pet");
    if (!pet) {
      throw new Error("NaviPetElement template is missing the pet element.");
    }
    this.#pet = pet;

    const resizeHandle = this.#root.querySelector<HTMLElement>(".resize-handle");
    if (!resizeHandle) {
      throw new Error("NaviPetElement template is missing the resize handle.");
    }
    this.#resizeHandle = resizeHandle;

    const speech = this.#root.querySelector<HTMLElement>(".speech");
    if (!speech) {
      throw new Error("NaviPetElement template is missing the speech element.");
    }
    this.#speech = speech;
  }

  connectedCallback() {
    this.#applyPosition();
    this.#syncSpeechFromAttributes();
    void this.#loadPet();
    this.#startAnimation();
    this.addEventListener("pointerdown", this.#handlePointerDown);
    this.addEventListener("pointermove", this.#handlePointerMove);
    this.addEventListener("pointerleave", this.#handlePointerLeave);
    this.addEventListener("pointerup", this.#handlePointerUp);
    this.addEventListener("pointercancel", this.#handlePointerUp);
  }

  disconnectedCallback() {
    this.#stopAnimation();
    this.removeEventListener("pointerdown", this.#handlePointerDown);
    this.removeEventListener("pointermove", this.#handlePointerMove);
    this.removeEventListener("pointerleave", this.#handlePointerLeave);
    this.removeEventListener("pointerup", this.#handlePointerUp);
    this.removeEventListener("pointercancel", this.#handlePointerUp);
    window.removeEventListener("pointermove", this.#handlePointerMove);
    window.removeEventListener("pointerup", this.#handlePointerUp);
    window.removeEventListener("pointercancel", this.#handlePointerUp);
    window.clearTimeout(this.#speechTimer);
  }

  attributeChangedCallback(name: string) {
    if (name === "position") {
      this.#applyPosition();
      return;
    }

    if (name === "pet" || name === "src") {
      void this.#loadPet();
      return;
    }

    if (name === "speech" || name === "speech-duration") {
      this.#syncSpeechFromAttributes();
      return;
    }

    // animation / state / fps: アニメ切替時にフレームをリセット
    this.#core = createPetCore();
  }

  say(text: string, options: NaviPetSpeechOptions = {}) {
    window.clearTimeout(this.#speechTimer);
    this.#speech.textContent = text;
    this.toggleAttribute("speech-open", text.length > 0);

    if (options.duration && options.duration > 0) {
      this.#speechTimer = window.setTimeout(() => this.clearSpeech(), options.duration);
    }
  }

  clearSpeech() {
    window.clearTimeout(this.#speechTimer);
    this.#speech.textContent = "";
    this.toggleAttribute("speech-open", false);
  }

  #requestedAnimation(): string | undefined {
    const explicit = this.getAttribute("animation");
    if (explicit) {
      return explicit;
    }
    const state = this.getAttribute("state");
    if (state && state in stateToAnimation) {
      return stateToAnimation[state];
    }
    return undefined;
  }

  #applyPosition() {
    if (this.hasAttribute("embedded")) {
      return;
    }

    const position = this.getAttribute("position") ?? "bottom-right";
    this.style.inset = "auto";

    if (position.includes("top")) {
      this.style.top = "24px";
    } else {
      this.style.bottom = "24px";
    }

    if (position.includes("left")) {
      this.style.left = "24px";
    } else {
      this.style.right = "24px";
    }
  }

  #applyFrame(animationName: string, frameIndex: number) {
    if (!this.#config) {
      return;
    }

    const animation = this.#getAnimation(animationName);
    const frame = animation.frames[frameIndex % animation.frames.length] ?? 0;
    const x = this.#toPercentage(frame, this.#config.columns);
    const y = this.#toPercentage(animation.row, this.#config.rows);

    this.style.setProperty("--navi-pet-frame-x", `${x}%`);
    this.style.setProperty("--navi-pet-frame-y", `${y}%`);
  }

  #getAnimation(name: string) {
    return this.#config?.animations[name] ?? this.#config?.animations.idle ?? defaultAnimations.idle;
  }

  #handlePointerDown = (event: PointerEvent) => {
    if (event.button !== 0) {
      return;
    }

    const rect = this.getBoundingClientRect();

    this.#pointerId = event.pointerId;
    this.#startPointerX = event.clientX;
    this.#startPointerY = event.clientY;
    this.#lastPointerX = event.clientX;
    this.#startLeft = rect.left;
    this.#startTop = rect.top;
    this.#startSize = rect.width;
    this.#resizing = event.composedPath().includes(this.#resizeHandle);
    this.setPointerCapture(event.pointerId);
    window.addEventListener("pointermove", this.#handlePointerMove);
    window.addEventListener("pointerup", this.#handlePointerUp);
    window.addEventListener("pointercancel", this.#handlePointerUp);
    this.toggleAttribute("dragging", !this.#resizing);
    this.toggleAttribute("resizing", this.#resizing);
    this.toggleAttribute("resize-handle", false);
    event.preventDefault();
  };

  #handlePointerMove = (event: PointerEvent) => {
    if (this.#pointerId === undefined) {
      this.toggleAttribute("resize-handle", this.#isInResizeZone(event));
      return;
    }

    if (event.pointerId !== this.#pointerId) {
      return;
    }

    if (this.#resizing) {
      this.#setSize(this.#startSize + event.clientX - this.#startPointerX);
      return;
    }

    const nextLeft = this.#startLeft + event.clientX - this.#startPointerX;
    const nextTop = this.#startTop + event.clientY - this.#startPointerY;
    const dragDeltaX = event.clientX - this.#lastPointerX;
    if (Math.abs(dragDeltaX) >= 1) {
      this.#dragDirectionX = Math.sign(dragDeltaX);
    }

    this.#lastPointerX = event.clientX;
    if (!this.hasAttribute("embedded")) {
      this.#setFixedPosition(nextLeft, nextTop);
    }
  };

  #handlePointerUp = (event: PointerEvent) => {
    if (event.pointerId !== this.#pointerId) {
      return;
    }

    this.releasePointerCapture(event.pointerId);
    this.#pointerId = undefined;
    this.#resizing = false;
    window.removeEventListener("pointermove", this.#handlePointerMove);
    window.removeEventListener("pointerup", this.#handlePointerUp);
    window.removeEventListener("pointercancel", this.#handlePointerUp);
    this.toggleAttribute("dragging", false);
    this.toggleAttribute("resizing", false);
  };

  #handlePointerLeave = () => {
    if (this.#pointerId === undefined) {
      this.toggleAttribute("resize-handle", false);
    }
  };

  #syncSpeechFromAttributes() {
    const text = this.getAttribute("speech") ?? "";
    const duration = Number(this.getAttribute("speech-duration")) || undefined;

    if (text) {
      this.say(text, { duration });
    } else {
      this.clearSpeech();
    }
  }

  async #loadPet() {
    if (!this.isConnected) {
      return;
    }

    const token = ++this.#loadToken;
    const config = await this.#resolveConfig();

    if (token !== this.#loadToken || !this.isConnected) {
      return;
    }

    this.#config = config;
    this.#core = createPetCore();
    this.#pet.setAttribute("aria-label", config.name);
    this.style.setProperty("--navi-pet-spritesheet", `url("${config.src}")`);
    this.style.setProperty("--navi-pet-columns", String(config.columns));
    this.style.setProperty("--navi-pet-rows", String(config.rows));

    if (config.frameWidth && config.frameHeight) {
      this.style.setProperty("--navi-pet-frame-ratio", `${config.frameWidth} / ${config.frameHeight}`);
    }

    this.#applyFrame("idle", 0);
    this.toggleAttribute("ready", true);
    this.dispatchEvent(new CustomEvent("navi-pet-load", { detail: config }));
  }

  async #resolveConfig(): Promise<RuntimePetConfig> {
    const petUrl = this.getAttribute("pet");
    const src = this.getAttribute("src");

    if (!petUrl) {
      return this.#normalizeManifest({ spritesheetPath: src ?? "" }, src ?? "", "navi pet");
    }

    const response = await fetch(petUrl);

    if (!response.ok) {
      throw new Error(`Unable to load navi pet manifest: ${response.status} ${response.statusText}`);
    }

    const manifest = (await response.json()) as PetManifest;
    return this.#normalizeManifest(
      manifest,
      new URL(manifest.spritesheetPath, response.url).href,
      manifest.displayName ?? manifest.id ?? "navi pet",
    );
  }

  #normalizeManifest(manifest: PetManifest, src: string, name: string): RuntimePetConfig {
    const columns = manifest.columns ?? 8;
    const rows = manifest.rows ?? 9;

    return {
      name,
      src,
      columns,
      rows,
      frameWidth: manifest.frameWidth,
      frameHeight: manifest.frameHeight,
      animations: this.#normalizeAnimations(manifest.animations ?? {}, columns, rows),
    };
  }

  #normalizeAnimations(animations: Record<string, AnimationConfig>, columns: number, rows: number) {
    const normalized: Record<string, Required<AnimationConfig>> = {};

    for (const [name, animation] of Object.entries({ ...defaultAnimations, ...animations })) {
      const row = this.#clamp(animation.row ?? 0, 0, rows - 1);
      const frames = (animation.frames?.length ? animation.frames : [...Array(columns).keys()]).map((frame) =>
        this.#clamp(frame, 0, columns - 1),
      );

      normalized[name] = {
        row,
        frames,
        fps: Math.max(1, animation.fps ?? 8),
      };
    }

    return normalized;
  }

  #startAnimation() {
    if (this.#animationFrame) {
      return;
    }

    const tick = (now: number) => {
      const frame = this.#core.tick({
        animations: this.#toCoreAnimations(),
        dragDirectionX: this.#dragDirectionX,
        dragging: this.#pointerId !== undefined,
        fpsOverride: Number(this.getAttribute("fps")) || undefined,
        now,
        requestedAnimation: this.#requestedAnimation(),
      });

      this.#applyFrame(frame.animation, frame.frameIndex);
      this.setAttribute("current-animation", frame.animation);

      this.#animationFrame = requestAnimationFrame(tick);
    };

    this.#animationFrame = requestAnimationFrame(tick);
  }

  #stopAnimation() {
    cancelAnimationFrame(this.#animationFrame);
    this.#animationFrame = 0;
  }

  #setFixedPosition(left: number, top: number) {
    const rect = this.getBoundingClientRect();
    const maxLeft = Math.max(0, window.innerWidth - rect.width);
    const maxTop = Math.max(0, window.innerHeight - rect.height);

    this.style.left = `${this.#clamp(left, 0, maxLeft)}px`;
    this.style.top = `${this.#clamp(top, 0, maxTop)}px`;
    this.style.right = "auto";
    this.style.bottom = "auto";
  }

  #setSize(size: number) {
    const nextSize = this.#clamp(size, 72, Math.min(window.innerWidth, window.innerHeight) * 0.8);
    this.style.setProperty("--navi-pet-size", `${nextSize}px`);
  }

  #isInResizeZone(event: PointerEvent, rect = this.getBoundingClientRect()) {
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    return x >= rect.width / 2 && y >= rect.height / 2;
  }

  #toPercentage(index: number, count: number) {
    return count <= 1 ? 0 : (index / (count - 1)) * 100;
  }

  #toCoreAnimations(): Record<string, CoreAnimation> {
    if (!this.#config) {
      return { idle: { fps: defaultAnimations.idle.fps, frameCount: defaultAnimations.idle.frames.length } };
    }

    return Object.fromEntries(
      Object.entries(this.#config.animations).map(([name, animation]) => [
        name,
        { fps: animation.fps, frameCount: animation.frames.length },
      ]),
    );
  }

  #clamp(value: number, min: number, max: number) {
    return Math.min(Math.max(value, min), max);
  }
}

if (!customElements.get("navi-pet")) {
  customElements.define("navi-pet", NaviPetElement);
}
