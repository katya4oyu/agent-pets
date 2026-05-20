import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";

type AgentState = "thinking" | "running" | "editing" | "waiting_approval" | "done" | "error";

type PetAsset = {
  id: string;
  displayName: string;
  description: string;
  spritesheetPath: string;
  spritesheetMime: string;
  spritesheetBytes: number[];
};

interface HookEventPayload {
  source: string;
  state: AgentState;
  label: string;
  message?: string;
  session_id?: string;
  cwd?: string;
  timestamp?: string;
}

interface AnimationSpec {
  row: number;
  frameCount: number;
  durations: number[];
}

const stateLabels: Record<AgentState, string> = {
  thinking: "Thinking",
  running: "Running",
  editing: "Editing",
  waiting_approval: "Waiting approval",
  done: "Ready",
  error: "Needs attention",
};

const animations: Record<AgentState, AnimationSpec> = {
  thinking:         { row: 8, frameCount: 6, durations: [150, 150, 150, 150, 150, 280] },
  running:          { row: 7, frameCount: 6, durations: [120, 120, 120, 120, 120, 220] },
  editing:          { row: 8, frameCount: 6, durations: [150, 150, 150, 150, 150, 280] },
  waiting_approval: { row: 3, frameCount: 4, durations: [140, 140, 140, 280] },
  done:             { row: 0, frameCount: 6, durations: [280, 110, 110, 140, 140, 320] },
  error:            { row: 5, frameCount: 8, durations: [140, 140, 140, 140, 140, 140, 140, 240] },
};

let animationTimer: ReturnType<typeof setTimeout> | null = null;
let spriteContext: CanvasRenderingContext2D | null = null;
let spriteImage: HTMLImageElement | null = null;

const app = document.querySelector<HTMLDivElement>("#app");
const currentWindow = getCurrentWindow();

if (!app) {
  throw new Error("Missing #app root");
}

app.innerHTML = `
  <section class="pet-shell" aria-label="Agent Pets status">
    <div class="speech-stack">
      <div class="speech" data-state="thinking" data-tauri-drag-region>
        <button class="speech-close" type="button" aria-label="Hide speech bubble">×</button>
        <span class="speech-status" aria-label="Agent is active"></span>
        <p class="speech-title">ffmpegで動画を10MB未満に圧縮</p>
        <p class="message">わかる、ffmpeg は便利だけど呪文感が強いです。 覚えるならまずこれだけで十分...</p>
        <div class="speech-actions">
          <button class="reply-open" type="button">Reply</button>
        </div>
        <form class="reply-form" aria-label="Reply to agent">
          <input class="reply-input" type="text" placeholder="Reply" />
          <button class="reply-submit" type="submit">Reply</button>
        </form>
      </div>
    </div>
    <div class="pet-wrap">
      <button class="bubble-toggle" type="button" aria-label="Hide speech bubble" aria-pressed="true">
        <span class="toggle-chevron" aria-hidden="true"></span>
        <span class="session-count" aria-label="1 active agent session">1</span>
      </button>
      <div class="pet" role="img" aria-label="Mio" data-tauri-drag-region>
        <canvas class="pet-sprite" width="192" height="208" aria-hidden="true" data-tauri-drag-region></canvas>
      </div>
      <button class="resize-handle" type="button" aria-label="Resize Agent Pets"></button>
    </div>
  </section>
`;

const shell = app.querySelector<HTMLElement>(".pet-shell");
const speechStack = app.querySelector<HTMLElement>(".speech-stack");
const speech = app.querySelector<HTMLElement>(".speech");
const closeButton = app.querySelector<HTMLButtonElement>(".speech-close");
const pet = app.querySelector<HTMLElement>(".pet");
const petWrap = app.querySelector<HTMLElement>(".pet-wrap");
const toggleButton = app.querySelector<HTMLButtonElement>(".bubble-toggle");
const resizeHandle = app.querySelector<HTMLButtonElement>(".resize-handle");
const replyOpen = app.querySelector<HTMLButtonElement>(".reply-open");
const replyForm = app.querySelector<HTMLFormElement>(".reply-form");
const replyInput = app.querySelector<HTMLInputElement>(".reply-input");
const speechTitle = app.querySelector<HTMLParagraphElement>(".speech-title");
const message = app.querySelector<HTMLParagraphElement>(".message");
const sprite = app.querySelector<HTMLCanvasElement>(".pet-sprite");

const frameWidth = 192;
const frameHeight = 208;
const defaultPetSize = 128;
const minPetSize = 64;
const maxPetSize = 256;
let speechVisible = true;

function setPetSize(size: number) {
  const nextSize = Math.round(Math.min(maxPetSize, Math.max(minPetSize, size)));
  shell?.style.setProperty("--pet-size", `${nextSize}px`);
}

function setSpeechVisible(nextVisible: boolean) {
  speechVisible = nextVisible;
  shell?.classList.toggle("speech-hidden", !speechVisible);
  shell?.classList.remove("replying");
  toggleButton?.setAttribute("aria-pressed", String(speechVisible));
  toggleButton?.setAttribute("aria-label", speechVisible ? "Hide speech bubble" : "Show speech bubble");
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

function applyAgentState(payload: HookEventPayload) {
  const spec = animations[payload.state] ?? animations.done;
  if (spriteContext && spriteImage) {
    setAnimation(spriteContext, spriteImage, spec);
  }
  if (speechTitle) speechTitle.textContent = payload.label;
  if (message) {
    message.textContent =
      payload.message ?? stateLabels[payload.state] ?? payload.label;
  }
}

async function loadMio() {
  if (!sprite) return;
  const context = sprite.getContext("2d");
  if (!context) return;

  try {
    const asset = await invoke<PetAsset>("load_pet_asset", { petId: "mio" });
    const image = new Image();
    const objectUrl = bytesToObjectUrl(asset);
    image.src = objectUrl;
    image.onload = () => {
      spriteContext = context;
      spriteImage = image;
      setAnimation(context, image, animations.done);
      if (speechTitle) speechTitle.textContent = asset.displayName;
      if (message) message.textContent = "is ready and waiting for your next prompt.";
    };
  } catch (error) {
    console.error(error);
    if (speechTitle) speechTitle.textContent = stateLabels.error;
    if (message) message.textContent = "Could not load Mio from ~/.codex/pets.";
  }
}

speech?.addEventListener("pointerdown", (event) => {
  const target = event.target;
  const interactive = target instanceof Element && Boolean(target.closest("button, input, form"));
  if (event.button === 0 && !interactive) {
    void currentWindow.startDragging();
  }
});

pet?.addEventListener("mousedown", (event) => {
  if (event.button === 0) {
    event.preventDefault();
    void currentWindow.startDragging();
  }
});

petWrap?.addEventListener("pointermove", (event) => {
  const bounds = petWrap.getBoundingClientRect();
  const inRightHalf = event.clientX >= bounds.left + bounds.width / 2;
  const inBottomHalf = event.clientY >= bounds.top + bounds.height / 2;
  petWrap.classList.toggle("resize-zone", inRightHalf && inBottomHalf);
});

petWrap?.addEventListener("pointerleave", () => {
  petWrap.classList.remove("resize-zone");
});

toggleButton?.addEventListener("pointerdown", (event) => {
  event.stopPropagation();
});

toggleButton?.addEventListener("click", (event) => {
  event.stopPropagation();
  setSpeechVisible(!speechVisible);
});

closeButton?.addEventListener("click", (event) => {
  event.stopPropagation();
  setSpeechVisible(false);
});

replyOpen?.addEventListener("click", (event) => {
  event.stopPropagation();
  shell?.classList.add("replying");
  window.setTimeout(() => replyInput?.focus(), 0);
});

replyForm?.addEventListener("pointerdown", (event) => {
  event.stopPropagation();
});

replyForm?.addEventListener("submit", (event) => {
  event.preventDefault();
  event.stopPropagation();
  const value = replyInput?.value.trim();
  if (!value) {
    replyInput?.focus();
    return;
  }
  if (message) {
    message.textContent = value;
  }
  if (replyInput) {
    replyInput.value = "";
  }
  shell?.classList.remove("replying");
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

setPetSize(defaultPetSize);
loadMio();

listen<HookEventPayload>("agent-state-changed", (event) => {
  applyAgentState(event.payload);
}).catch(console.error);
