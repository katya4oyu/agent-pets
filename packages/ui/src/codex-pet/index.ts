// codex-pet — the codex-compatible sprite renderer.
//
// Importing this module registers the `<navi-pet>` custom element as a side
// effect (see navi-pet.ts). Depends only on the codex pet atlas convention
// (docs/codex-pet-spritesheets.md); no Tauri, Vite, or navi-shell coupling.
export { NaviPetElement } from "./navi-pet";
export type { NaviPetSpeechOptions } from "./navi-pet";
export { createPetCore } from "./pet-core";
export type { CoreAnimation, PetCore, PetCoreInput, AnimationPlayback } from "./pet-core";
