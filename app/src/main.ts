import { invoke, listen, startDragging } from "./bridge";
import "./styles.css";
import {
  type AgentState,
  type DisplayMode,
  type StatusCardData,
  highestPriorityState,
  isVisibleInAuto,
  createStatusCard,
  updateStatusCard,
} from "@navi/ui/navi";
import {
  type HookEventPayload,
  type AnimationSpec,
  animations,
  sessionKey,
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

interface DisplayModePayload {
  mode: DisplayMode;
}

interface SessionEntry {
  state: AgentState;
  element: HTMLElement;
}

let animationTimer: ReturnType<typeof setTimeout> | null = null;
let spriteContext: CanvasRenderingContext2D | null = null;
let spriteImage: HTMLImageElement | null = null;
let spriteObjectUrl: string | null = null;

const sessions = new Map<string, SessionEntry>();

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing #app root");
}

app.innerHTML = `
  <section class="pet-shell" aria-label="navi status">
    <div class="status-stack">
    </div>
    <div class="pet-wrap">
      <button class="status-toggle" type="button" aria-label="Hide status cards" aria-pressed="true">
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
const statusStack = app.querySelector<HTMLElement>(".status-stack");
const pet = app.querySelector<HTMLElement>(".pet");
const petWrap = app.querySelector<HTMLElement>(".pet-wrap");
const toggleButton = app.querySelector<HTMLButtonElement>(".status-toggle");
const resizeHandle = app.querySelector<HTMLButtonElement>(".resize-handle");
const setupBtn = app.querySelector<HTMLButtonElement>(".setup-btn");
const sessionCountEl = app.querySelector<HTMLSpanElement>(".session-count");
const sprite = app.querySelector<HTMLCanvasElement>(".pet-sprite");

const frameWidth = 192;
const frameHeight = 208;
const defaultPetSize = 128;
const minPetSize = 64;
const maxPetSize = 256;
let cardsVisible = true;
let displayMode: DisplayMode = "show";

function setPetSize(size: number) {
  const nextSize = Math.round(Math.min(maxPetSize, Math.max(minPetSize, size)));
  shell?.style.setProperty("--pet-size", `${nextSize}px`);
}

function setCardsVisible(nextVisible: boolean) {
  cardsVisible = nextVisible;
  shell?.classList.toggle("status-hidden", !cardsVisible);
  toggleButton?.setAttribute("aria-pressed", String(cardsVisible));
  toggleButton?.setAttribute("aria-label", cardsVisible ? "Hide status cards" : "Show status cards");
}

function applyDisplayMode(mode: DisplayMode) {
  displayMode = mode;
  if (displayMode === "show") {
    setCardsVisible(true);
  } else if (displayMode === "hide") {
    setCardsVisible(false);
  } else {
    setCardsVisible(isVisibleInAuto(getHighestPriorityState()));
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

/** hook ペイロード → ステータスカードの表示データ（project_name → project）。 */
function toCardData(payload: HookEventPayload): StatusCardData {
  return {
    source: payload.source,
    state: payload.state,
    label: payload.label,
    message: payload.message,
    project: payload.project_name,
    cwd: payload.cwd,
  };
}

/** @navi/ui のステータスカードを生成し、シェル固有の配線（Tauri ドラッグ）を足して載せる。 */
function mountStatusCard(key: string, data: StatusCardData): HTMLElement {
  const card = createStatusCard(key, data, { onClose: removeStatusCard });
  card.setAttribute("data-tauri-drag-region", "");
  card.addEventListener("pointerdown", (event) => {
    const target = event.target;
    const interactive = target instanceof Element && Boolean(target.closest("button"));
    if (event.button === 0 && !interactive) {
      void startDragging();
    }
  });
  statusStack?.appendChild(card);
  return card;
}

function removeStatusCard(key: string) {
  const session = sessions.get(key);
  if (!session) return;
  session.element.remove();
  sessions.delete(key);
  updateSessionCount();
  updatePetAnimation();
  if (displayMode === "auto") {
    setCardsVisible(isVisibleInAuto(getHighestPriorityState()));
  }
}

function applyAgentState(payload: HookEventPayload) {
  const key = sessionKey(payload);
  const data = toCardData(payload);

  let session = sessions.get(key);
  if (!session) {
    const element = mountStatusCard(key, data);
    session = { state: payload.state, element };
    sessions.set(key, session);
  } else {
    updateStatusCard(session.element, data);
  }
  session.state = payload.state;
  updateSessionCount();
  updatePetAnimation();
  if (displayMode === "auto") {
    setCardsVisible(isVisibleInAuto(getHighestPriorityState()));
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
    void startDragging();
  }
});

toggleButton?.addEventListener("pointerdown", (event) => {
  event.stopPropagation();
});

toggleButton?.addEventListener("click", (event) => {
  event.stopPropagation();
  displayMode = cardsVisible ? "hide" : "show";
  setCardsVisible(!cardsVisible);
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

listen<HookEventPayload>("agent-state-changed", (payload) => {
  applyAgentState(payload);
}).catch(console.error);

listen<PetSizePayload>("set-pet-size", (payload) => {
  setPetSize(payload.size);
}).catch(console.error);

listen<PetSelectionPayload>("set-pet", (payload) => {
  void loadPet(payload.petId);
}).catch(console.error);

listen<DisplayModePayload>("set-speech-mode", (payload) => {
  applyDisplayMode(payload.mode);
}).catch(console.error);
