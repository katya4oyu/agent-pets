import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import claudeCodeSvg from "@lobehub/icons-static-svg/icons/claudecode.svg?raw";
import codexSvg from "@lobehub/icons-static-svg/icons/codex.svg?raw";
import copilotSvg from "@lobehub/icons-static-svg/icons/copilot.svg?raw";
import {
  type AgentState,
  type SpeechMode,
  type HookEventPayload,
  type AnimationSpec,
  animations,
  sessionKey,
  highestPriorityState,
  bubbleMessage,
  bubbleDir,
  isSpeechVisibleInAuto,
} from "./state";

type PetAsset = {
  id: string;
  displayName: string;
  description: string;
  spritesheetPath: string;
  spritesheetMime: string;
  spritesheetBytes: number[];
};

interface PetSizePayload {
  size: number;
}

interface PetSelectionPayload {
  petId: string;
}

interface SpeechModePayload {
  mode: SpeechMode;
}

interface SessionEntry {
  state: AgentState;
  element: HTMLElement;
}

const sourceConfig: Record<string, { label: string; color: string; svg: string }> = {
  "claude-code": { label: "Claude Code", color: "#CC785C", svg: claudeCodeSvg },
  codex:         { label: "Codex",       color: "#10A37F", svg: codexSvg },
  copilot:       { label: "Copilot",     color: "#6F42C1", svg: copilotSvg },
};

let animationTimer: ReturnType<typeof setTimeout> | null = null;
let spriteContext: CanvasRenderingContext2D | null = null;
let spriteImage: HTMLImageElement | null = null;
let spriteObjectUrl: string | null = null;

const sessions = new Map<string, SessionEntry>();

const app = document.querySelector<HTMLDivElement>("#app");
const currentWindow = getCurrentWindow();

if (!app) {
  throw new Error("Missing #app root");
}

app.innerHTML = `
  <section class="pet-shell" aria-label="navi status">
    <div class="speech-stack">
    </div>
    <div class="pet-wrap">
      <button class="bubble-toggle" type="button" aria-label="Hide speech bubble" aria-pressed="true">
        <span class="toggle-chevron" aria-hidden="true"></span>
        <span class="session-count" aria-label="0 active agent sessions">0</span>
      </button>
      <div class="pet" role="img" aria-label="Mio" data-tauri-drag-region>
        <canvas class="pet-sprite" width="192" height="208" aria-hidden="true" data-tauri-drag-region></canvas>
      </div>
      <button class="resize-handle" type="button" aria-label="Resize navi"></button>
      <button class="setup-btn" type="button" aria-label="Setup hooks">⚙</button>
    </div>
  </section>
`;

const shell = app.querySelector<HTMLElement>(".pet-shell");
const speechStack = app.querySelector<HTMLElement>(".speech-stack");
const pet = app.querySelector<HTMLElement>(".pet");
const petWrap = app.querySelector<HTMLElement>(".pet-wrap");
const toggleButton = app.querySelector<HTMLButtonElement>(".bubble-toggle");
const resizeHandle = app.querySelector<HTMLButtonElement>(".resize-handle");
const setupBtn = app.querySelector<HTMLButtonElement>(".setup-btn");
const sessionCountEl = app.querySelector<HTMLSpanElement>(".session-count");
const sprite = app.querySelector<HTMLCanvasElement>(".pet-sprite");

const frameWidth = 192;
const frameHeight = 208;
const defaultPetSize = 128;
const minPetSize = 64;
const maxPetSize = 256;
let speechVisible = true;
let speechMode: SpeechMode = "show";

function setPetSize(size: number) {
  const nextSize = Math.round(Math.min(maxPetSize, Math.max(minPetSize, size)));
  shell?.style.setProperty("--pet-size", `${nextSize}px`);
}

function setSpeechVisible(nextVisible: boolean) {
  speechVisible = nextVisible;
  shell?.classList.toggle("speech-hidden", !speechVisible);
  toggleButton?.setAttribute("aria-pressed", String(speechVisible));
  toggleButton?.setAttribute("aria-label", speechVisible ? "Hide speech bubble" : "Show speech bubble");
}

function applySpeechMode(mode: SpeechMode) {
  speechMode = mode;
  if (speechMode === "show") {
    setSpeechVisible(true);
  } else if (speechMode === "hide") {
    setSpeechVisible(false);
  } else {
    setSpeechVisible(isSpeechVisibleInAuto(getHighestPriorityState()));
  }
}

function bytesToObjectUrl(asset: PetAsset): string {
  const bytes = new Uint8Array(asset.spritesheetBytes);
  const blob = new Blob([bytes], { type: asset.spritesheetMime });
  return URL.createObjectURL(blob);
}

function drawFrame(context: CanvasRenderingContext2D, image: HTMLImageElement, column: number, row: number) {
  context.clearRect(0, 0, frameWidth, frameHeight);
  context.drawImage(
    image,
    column * frameWidth,
    row * frameHeight,
    frameWidth,
    frameHeight,
    0,
    0,
    frameWidth,
    frameHeight,
  );
}

function setAnimation(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  spec: AnimationSpec,
) {
  if (animationTimer !== null) {
    clearTimeout(animationTimer);
    animationTimer = null;
  }
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reducedMotion) {
    drawFrame(context, image, 0, spec.row);
    return;
  }
  let frame = 0;
  const tick = () => {
    drawFrame(context, image, frame, spec.row);
    const duration = spec.durations[frame] ?? spec.durations[spec.durations.length - 1] ?? 180;
    frame = (frame + 1) % spec.frameCount;
    animationTimer = setTimeout(tick, duration);
  };
  tick();
}

function getHighestPriorityState(): AgentState {
  return highestPriorityState(Array.from(sessions.values(), (entry) => entry.state));
}

function updatePetAnimation() {
  const state = getHighestPriorityState();
  const spec = animations[state] ?? animations.done;
  if (spriteContext && spriteImage) {
    setAnimation(spriteContext, spriteImage, spec);
  }
}

function updateSessionCount() {
  const count = sessions.size;
  if (sessionCountEl) {
    sessionCountEl.textContent = String(count);
    sessionCountEl.setAttribute(
      "aria-label",
      `${count} active agent session${count !== 1 ? "s" : ""}`,
    );
  }
}

function createBubbleElement(key: string): HTMLElement {
  const bubble = document.createElement("div");
  bubble.className = "speech";
  bubble.setAttribute("data-tauri-drag-region", "");
  bubble.innerHTML = `
    <button class="speech-close" type="button" aria-label="Remove session">×</button>
    <div class="source-badge" aria-label="Source agent"></div>
    <p class="speech-title"></p>
    <p class="message"></p>
    <p class="cwd-label" hidden></p>
  `;
  bubble.querySelector<HTMLButtonElement>(".speech-close")?.addEventListener("click", (e) => {
    e.stopPropagation();
    removeBubble(key);
  });
  bubble.addEventListener("pointerdown", (event) => {
    const target = event.target;
    const interactive = target instanceof Element && Boolean(target.closest("button"));
    if (event.button === 0 && !interactive) {
      void currentWindow.startDragging();
    }
  });
  speechStack?.appendChild(bubble);
  return bubble;
}

function updateBubbleElement(bubble: HTMLElement, payload: HookEventPayload) {
  bubble.setAttribute("data-state", payload.state);
  const title = bubble.querySelector<HTMLParagraphElement>(".speech-title");
  const msg = bubble.querySelector<HTMLParagraphElement>(".message");
  const cwdLabel = bubble.querySelector<HTMLParagraphElement>(".cwd-label");
  const sourceBadge = bubble.querySelector<HTMLDivElement>(".source-badge");

  if (title) title.textContent = payload.label;
  if (msg) {
    msg.textContent = bubbleMessage(payload);
  }
  const dir = bubbleDir(payload);
  if (cwdLabel) {
    cwdLabel.textContent = dir ?? "";
    cwdLabel.hidden = !dir;
  }
  const cfg = sourceConfig[payload.source];
  if (sourceBadge) {
    if (cfg) {
      sourceBadge.innerHTML = cfg.svg;
      sourceBadge.setAttribute("data-source", payload.source);
      sourceBadge.title = cfg.label;
    } else {
      sourceBadge.innerHTML = "";
      sourceBadge.removeAttribute("data-source");
      sourceBadge.title = payload.source;
    }
  }
}

function removeBubble(key: string) {
  const session = sessions.get(key);
  if (!session) return;
  session.element.remove();
  sessions.delete(key);
  updateSessionCount();
  updatePetAnimation();
  if (speechMode === "auto") {
    setSpeechVisible(isSpeechVisibleInAuto(getHighestPriorityState()));
  }
}

function applyAgentState(payload: HookEventPayload) {
  const key = sessionKey(payload);

  let session = sessions.get(key);
  if (!session) {
    const element = createBubbleElement(key);
    session = { state: payload.state, element };
    sessions.set(key, session);
  }
  session.state = payload.state;
  updateBubbleElement(session.element, payload);
  updateSessionCount();
  updatePetAnimation();
  if (speechMode === "auto") {
    setSpeechVisible(isSpeechVisibleInAuto(getHighestPriorityState()));
  }
}

async function loadPet(petId = "mio") {
  if (!sprite) return;
  const context = sprite.getContext("2d");
  if (!context) return;

  try {
    const asset = await invoke<PetAsset>("load_pet_asset", { petId });
    const image = new Image();
    const objectUrl = bytesToObjectUrl(asset);
    if (spriteObjectUrl) {
      URL.revokeObjectURL(spriteObjectUrl);
    }
    spriteObjectUrl = objectUrl;
    image.src = objectUrl;
    image.onload = () => {
      spriteContext = context;
      spriteImage = image;
      setAnimation(context, image, animations.done);
    };
  } catch (error) {
    console.error(error);
  }
}

petWrap?.addEventListener("pointermove", (event) => {
  const bounds = petWrap.getBoundingClientRect();
  const inRightHalf = event.clientX >= bounds.left + bounds.width / 2;
  const inBottomHalf = event.clientY >= bounds.top + bounds.height / 2;
  petWrap.classList.toggle("resize-zone", inRightHalf && inBottomHalf);
});

petWrap?.addEventListener("pointerleave", () => {
  petWrap.classList.remove("resize-zone");
});

pet?.addEventListener("mousedown", (event) => {
  if (event.button === 0) {
    event.preventDefault();
    void currentWindow.startDragging();
  }
});

toggleButton?.addEventListener("pointerdown", (event) => {
  event.stopPropagation();
});

toggleButton?.addEventListener("click", (event) => {
  event.stopPropagation();
  speechMode = speechVisible ? "hide" : "show";
  setSpeechVisible(!speechVisible);
});

resizeHandle?.addEventListener("pointerdown", (event) => {
  event.preventDefault();
  event.stopPropagation();
  const startX = event.clientX;
  const startY = event.clientY;
  const startSize = petWrap?.getBoundingClientRect().width ?? defaultPetSize;
  resizeHandle.setPointerCapture(event.pointerId);

  const resize = (moveEvent: PointerEvent) => {
    const delta = Math.max(moveEvent.clientX - startX, moveEvent.clientY - startY);
    setPetSize(startSize + delta);
  };
  const finish = (upEvent: PointerEvent) => {
    resizeHandle.releasePointerCapture(upEvent.pointerId);
    window.removeEventListener("pointermove", resize);
    window.removeEventListener("pointerup", finish);
  };

  window.addEventListener("pointermove", resize);
  window.addEventListener("pointerup", finish);
});

setupBtn?.addEventListener("click", async (event) => {
  event.stopPropagation();
  setupBtn.disabled = true;
  try {
    await invoke<string>("setup_hooks", { agent: "all" });
  } catch (e) {
    console.error(e);
  } finally {
    setupBtn.disabled = false;
  }
});

setPetSize(defaultPetSize);
loadPet();

listen<HookEventPayload>("agent-state-changed", (event) => {
  applyAgentState(event.payload);
}).catch(console.error);

listen<PetSizePayload>("set-pet-size", (event) => {
  setPetSize(event.payload.size);
}).catch(console.error);

listen<PetSelectionPayload>("set-pet", (event) => {
  void loadPet(event.payload.petId);
}).catch(console.error);

listen<SpeechModePayload>("set-speech-mode", (event) => {
  applySpeechMode(event.payload.mode);
}).catch(console.error);
