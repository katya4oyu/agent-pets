import "./pet/navi-pet";

// 各 AgentState がどのアニメ（行）に対応するかを一覧で検証するための使い捨てページ。
const stateToAnimation: Record<string, string> = {
  done: "idle",
  thinking: "review",
  running: "running",
  editing: "review",
  waiting_approval: "waving",
  error: "failed",
};

const root = document.querySelector<HTMLElement>("#gallery");
if (!root) {
  throw new Error("Missing #gallery root");
}

root.innerHTML = Object.entries(stateToAnimation)
  .map(
    ([state, anim]) => `
      <figure>
        <navi-pet embedded pet="/pets/mio/pet.json" state="${state}" style="--navi-pet-size:128px"></navi-pet>
        <figcaption>${state} <small>→ ${anim}</small></figcaption>
      </figure>`,
  )
  .join("");
