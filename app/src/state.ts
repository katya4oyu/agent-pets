// App-shell domain logic for the pet overlay (sprite + hook payload + session keying).
//
// Pure presentation logic and the navi UI state model now live in `@navi/ui`
// (`packages/ui/src/navi/state.ts`) so the app shell and the playground share one
// source of truth. This module keeps only what is app-specific:
//   - the hook payload shape (Rust protocol),
//   - the sprite animation table (codex atlas rows),
//   - session keying.
// It re-exports the navi state model so existing `./state` importers keep working.

export {
  type AgentState,
  type SourceId,
  type DisplayMode,
  type StatusCardData,
  stateLabels,
  STATE_PRIORITY,
  agentStates,
  highestPriorityState,
  cardMessage,
  cardDir,
  isVisibleInAuto,
} from "@navi/ui/navi";

import type { AgentState } from "@navi/ui/navi";

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
