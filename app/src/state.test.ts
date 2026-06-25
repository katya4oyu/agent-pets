import { describe, it, expect } from "vitest";
import {
  type AgentState,
  type HookEventPayload,
  stateLabels,
  STATE_PRIORITY,
  animations,
  sessionKey,
  highestPriorityState,
  cardMessage,
  cardDir,
  isVisibleInAuto,
} from "./state";

const ALL_STATES: AgentState[] = [
  "thinking",
  "running",
  "editing",
  "waiting_approval",
  "done",
  "error",
];

function event(partial: Partial<HookEventPayload>): HookEventPayload {
  return { source: "codex", state: "running", label: "Running", ...partial };
}

describe("sessionKey", () => {
  it("combines source and session id when a session id is present", () => {
    expect(sessionKey({ source: "claude-code", session_id: "abc" })).toBe(
      "claude-code:abc",
    );
  });

  it("falls back to the source alone when there is no session id", () => {
    expect(sessionKey({ source: "codex" })).toBe("codex");
    expect(sessionKey({ source: "codex", session_id: undefined })).toBe("codex");
  });

  it("does not collapse different sessions of the same source", () => {
    expect(sessionKey({ source: "codex", session_id: "1" })).not.toBe(
      sessionKey({ source: "codex", session_id: "2" }),
    );
  });
});

describe("highestPriorityState", () => {
  it("resolves to done when there are no active sessions", () => {
    expect(highestPriorityState([])).toBe("done");
  });

  it("returns the only state when a single session is active", () => {
    expect(highestPriorityState(["running"])).toBe("running");
  });

  it("returns done when every active session is done", () => {
    expect(highestPriorityState(["done", "done"])).toBe("done");
  });

  it("prefers error over everything else", () => {
    expect(highestPriorityState(["done", "running", "error", "thinking"])).toBe(
      "error",
    );
  });

  it("prefers waiting_approval over thinking/running/editing/done", () => {
    expect(
      highestPriorityState(["done", "editing", "running", "thinking", "waiting_approval"]),
    ).toBe("waiting_approval");
  });

  it("follows the documented priority ordering", () => {
    // error > waiting_approval > thinking > running > editing > done
    const ordered: AgentState[] = [
      "error",
      "waiting_approval",
      "thinking",
      "running",
      "editing",
      "done",
    ];
    for (let i = 0; i < ordered.length - 1; i += 1) {
      expect(STATE_PRIORITY[ordered[i]]).toBeGreaterThan(
        STATE_PRIORITY[ordered[i + 1]],
      );
      // The higher-priority state must win regardless of input order.
      expect(highestPriorityState([ordered[i + 1], ordered[i]])).toBe(ordered[i]);
    }
  });
});

describe("cardMessage", () => {
  it("uses the explicit message when present", () => {
    expect(cardMessage(event({ message: "cargo test" }))).toBe("cargo test");
  });

  it("falls back to the human label for the state when no message is given", () => {
    expect(cardMessage(event({ state: "waiting_approval", message: undefined }))).toBe(
      stateLabels.waiting_approval,
    );
    expect(cardMessage(event({ state: "done" }))).toBe(stateLabels.done);
  });

  it("falls back to the raw label for an unknown state", () => {
    const weird = event({
      state: "mystery" as AgentState,
      label: "Pondering",
      message: undefined,
    });
    expect(cardMessage(weird)).toBe("Pondering");
  });
});

describe("cardDir", () => {
  it("prefers the project name when present", () => {
    expect(cardDir({ project: "agent-pets", cwd: "/home/user/x" })).toBe(
      "agent-pets",
    );
  });

  it("uses the last path segment of cwd when there is no project name", () => {
    expect(cardDir({ cwd: "/home/user/projects/navi" })).toBe("navi");
  });

  it("ignores trailing slashes in cwd", () => {
    expect(cardDir({ cwd: "/home/user/projects/navi/" })).toBe("navi");
  });

  it("returns null when neither project name nor cwd is available", () => {
    expect(cardDir({})).toBeNull();
    expect(cardDir({ cwd: "" })).toBeNull();
  });
});

describe("isVisibleInAuto", () => {
  it("hides the cards only when the aggregate state is done", () => {
    expect(isVisibleInAuto("done")).toBe(false);
    for (const state of ALL_STATES.filter((s) => s !== "done")) {
      expect(isVisibleInAuto(state)).toBe(true);
    }
  });
});

describe("static tables", () => {
  it("defines a label and an animation for every state", () => {
    for (const state of ALL_STATES) {
      expect(typeof stateLabels[state]).toBe("string");
      expect(animations[state]).toBeDefined();
    }
  });

  it("keeps each animation's duration count aligned with its frame count", () => {
    for (const state of ALL_STATES) {
      expect(animations[state].durations).toHaveLength(animations[state].frameCount);
    }
  });
});
