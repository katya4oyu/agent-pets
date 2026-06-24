import "@navi/ui";
import "./playground.css";

type AgentState = "done" | "thinking" | "running" | "editing" | "waiting_approval" | "error";

const states: AgentState[] = [
  "done",
  "thinking",
  "running",
  "editing",
  "waiting_approval",
  "error",
];

const root = document.querySelector<HTMLElement>("#playground");
if (!root) {
  throw new Error("Missing #playground root");
}

root.innerHTML = `
  <div class="pg-stage">
    <navi-pet embedded pet="/pets/mio/pet.json" state="done"></navi-pet>
  </div>
  <aside class="pg-panel">
    <h1>navi playground</h1>
    <p class="pg-sub">Tauri 不要・ブラウザだけで見た目を確認</p>

    <h2>Agent state</h2>
    <div class="pg-states"></div>

    <h2>Pet size</h2>
    <input type="range" min="64" max="256" value="160" id="pg-size" />

    <p class="pg-current">
      state: <code id="pg-state-label">done</code><br />
      anim: <code id="pg-anim-label">–</code>
    </p>

    <p class="pg-hint">
      ペットはドラッグで移動、右下でリサイズできます。<br />
      次の段階で吹き出し（マルチセッション）をここに重ねます。
    </p>
  </aside>
`;

const pet = root.querySelector<HTMLElement>("navi-pet");
const statesEl = root.querySelector<HTMLElement>(".pg-states");
const stateLabel = root.querySelector<HTMLElement>("#pg-state-label");
const animLabel = root.querySelector<HTMLElement>("#pg-anim-label");
const sizeInput = root.querySelector<HTMLInputElement>("#pg-size");

if (!pet || !statesEl || !stateLabel || !animLabel || !sizeInput) {
  throw new Error("playground: failed to build controls");
}

pet.style.setProperty("--navi-pet-size", "160px");

const buttons = new Map<AgentState, HTMLButtonElement>();

function selectState(next: AgentState) {
  pet!.setAttribute("state", next);
  stateLabel!.textContent = next;
  for (const [key, btn] of buttons) {
    btn.setAttribute("aria-pressed", String(key === next));
  }
}

for (const state of states) {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.textContent = state;
  btn.setAttribute("aria-pressed", String(state === "done"));
  btn.addEventListener("click", () => selectState(state));
  buttons.set(state, btn);
  statesEl.appendChild(btn);
}

sizeInput.addEventListener("input", () => {
  pet.style.setProperty("--navi-pet-size", `${sizeInput.value}px`);
});

// navi-pet が毎フレーム書き込む current-animation を表示に反映
const observer = new MutationObserver(() => {
  animLabel.textContent = pet.getAttribute("current-animation") ?? "–";
});
observer.observe(pet, { attributes: true, attributeFilter: ["current-animation"] });
