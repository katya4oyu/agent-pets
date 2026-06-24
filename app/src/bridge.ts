// Tauri abstraction layer ("bridge").
//
// The navi shell (`main.ts`) talks to the Rust backend through Tauri's
// `invoke` / `listen` / window APIs. To let the same frontend run *without*
// Tauri — in a plain browser, on Vite preview, or deployed to Cloudflare for
// visual verification — every Tauri touch point is funnelled through here.
//
// At runtime we pick one of two implementations:
//   - Tauri present (`window.__TAURI_INTERNALS__`): forward to `@tauri-apps/api`.
//   - Otherwise: a self-contained browser mock that serves the bundled pet
//     assets over `fetch`, so the pet still renders for debugging.
//
// `@tauri-apps/api` is imported statically here, but this module is only part
// of the Tauri/`main.ts` bundle — the Cloudflare web build ships the
// `playground` entry instead, which never imports this file.

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen as tauriListen } from "@tauri-apps/api/event";

export interface PetAsset {
  id: string;
  displayName: string;
  description: string;
  spritesheetPath: string;
  spritesheetMime: string;
  spritesheetBytes: number[];
}

export type UnlistenFn = () => void;

/** True when running inside the actual Tauri webview. */
export const isTauri: boolean =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Call a Rust command. Falls back to a browser mock outside Tauri. */
export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return isTauri ? tauriInvoke<T>(cmd, args) : mockInvoke<T>(cmd, args);
}

/**
 * Subscribe to a backend event. Returns a function that removes the listener.
 * Outside Tauri no backend exists, so this is a no-op subscription.
 */
export function listen<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (isTauri) {
    return tauriListen<T>(event, (e) => handler(e.payload));
  }
  return Promise.resolve(() => {});
}

// `getCurrentWindow()` reads window metadata from the Tauri runtime, so only
// resolve it lazily when we know Tauri is present.
let cachedWindow: ReturnType<typeof getCurrentWindow> | null = null;

/** Start an OS window drag from the current pointer (no-op outside Tauri). */
export function startDragging(): Promise<void> {
  if (!isTauri) return Promise.resolve();
  cachedWindow ??= getCurrentWindow();
  return cachedWindow.startDragging();
}

// ── Browser mock ─────────────────────────────────────────────────────────────

interface PetManifest {
  id?: string;
  displayName?: string;
  description?: string;
  spritesheetPath: string;
}

async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case "load_pet_asset": {
      const petId = (args?.petId as string | undefined) ?? "mio";
      return (await loadMockPetAsset(petId)) as T;
    }
    case "setup_hooks":
    case "install_cli_tool":
      return "Hooks are only available inside the navi desktop app." as unknown as T;
    default:
      console.warn(`[navi bridge] unhandled mock command: ${cmd}`);
      return undefined as T;
  }
}

// Serve the pet straight from the bundled `public/pets/<id>/` assets so the
// canvas renderer in `main.ts` works unchanged when Tauri is absent.
async function loadMockPetAsset(petId: string): Promise<PetAsset> {
  const dir = `${import.meta.env.BASE_URL}pets/${petId}/`;

  const manifestResponse = await fetch(`${dir}pet.json`);
  if (!manifestResponse.ok) {
    throw new Error(`Mock pet "${petId}" not found (${manifestResponse.status}).`);
  }
  const manifest = (await manifestResponse.json()) as PetManifest;

  const spriteUrl = new URL(manifest.spritesheetPath, new URL(dir, location.href)).href;
  const spriteResponse = await fetch(spriteUrl);
  if (!spriteResponse.ok) {
    throw new Error(`Mock spritesheet for "${petId}" not found (${spriteResponse.status}).`);
  }
  const bytes = new Uint8Array(await spriteResponse.arrayBuffer());

  return {
    id: manifest.id ?? petId,
    displayName: manifest.displayName ?? petId,
    description: manifest.description ?? "",
    spritesheetPath: manifest.spritesheetPath,
    spritesheetMime: spriteResponse.headers.get("content-type") ?? "image/webp",
    spritesheetBytes: Array.from(bytes),
  };
}
