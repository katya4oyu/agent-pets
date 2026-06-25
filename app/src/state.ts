// Pure, framework-free presentation logic for the pet overlay.
//
// This module intentionally has NO dependency on the DOM, Tauri, or any
// Vite-specific import (e.g. `?raw` assets). Everything here is a pure value
// or pure function so it can be unit-tested headlessly with vitest and, later,
// moved into / mirrored by the backend World Model without behavior changes.
//
// `main.ts` imports from here; keep the behavior of these functions identical
// to what `main.ts` previously did inline (these are characterization targets).

export type AgentState =
  | "thinking"
  | "running"
  | "editing"
  | "waiting_approval"
  | "done"
  | "error";

export type DisplayMode = "show" | "hide" | "auto";

export interface HookEventPayload {
  source: string;
  state: AgentState;
  label: string;
  message?: string;
  session_id?: string;
  cwd?: string;
  project_name?: string;
  timestamp?: string;
  terminal_program?: string;
  terminal_session_id?: string;
}

export interface AnimationSpec {
  row: number;
  frameCount: number;
  durations: number[];
}

export const stateLabels: Record<AgentState, string> = {
  thinking: "Thinking",
  running: "Running",
  editing: "Editing",
  waiting_approval: "Waiting approval",
  done: "Ready",
  error: "Needs attention",
};

// Higher number wins when several sessions are active at once.
export const STATE_PRIORITY: Record<AgentState, number> = {
  error: 6,
  waiting_approval: 5,
  thinking: 4,
  running: 3,
  editing: 2,
  done: 1,
};

export const animations: Record<AgentState, AnimationSpec> = {
  thinking:         { row: 8, frameCount: 6, durations: [150, 150, 150, 150, 150, 280] },
  running:          { row: 7, frameCount: 6, durations: [120, 120, 120, 120, 120, 220] },
  editing:          { row: 8, frameCount: 6, durations: [150, 150, 150, 150, 150, 280] },
  waiting_approval: { row: 3, frameCount: 4, durations: [140, 140, 140, 280] },
  done:             { row: 0, frameCount: 6, durations: [280, 110, 110, 140, 140, 320] },
  error:            { row: 5, frameCount: 8, durations: [140, 140, 140, 140, 140, 140, 140, 240] },
};

/**
 * Stable key used to group events into a single status card/session.
 * Falls back to the source name when no session id is provided.
 */
export function sessionKey(
  payload: Pick<HookEventPayload, "source" | "session_id">,
): string {
  return payload.session_id
    ? `${payload.source}:${payload.session_id}`
    : payload.source;
}

/**
 * The most "important" state across all active sessions, used to drive the
 * single pet animation. Empty input (no active sessions) resolves to "done".
 */
export function highestPriorityState(states: Iterable<AgentState>): AgentState {
  let best: AgentState = "done";
  for (const state of states) {
    if (STATE_PRIORITY[state] > STATE_PRIORITY[best]) {
      best = state;
    }
  }
  return best;
}

/** Text shown in a status card: explicit message, else a label for the state. */
export function cardMessage(payload: HookEventPayload): string {
  return payload.message ?? stateLabels[payload.state] ?? payload.label;
}

/** Short directory label: prefer project name, else the last path segment. */
export function cardDir(
  payload: Pick<HookEventPayload, "project_name" | "cwd">,
): string | null {
  return (
    payload.project_name ??
    (payload.cwd ? payload.cwd.split("/").filter(Boolean).at(-1) ?? null : null)
  );
}

/** In "auto" display mode, the cards are shown unless everything is done. */
export function isVisibleInAuto(highest: AgentState): boolean {
  return highest !== "done";
}
