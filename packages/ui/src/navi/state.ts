// navi 固有 UI のプレゼンテーション・ロジック（DOM/Tauri 非依存・純関数）。
//
// codex-pet（スプライト描画）とは出自が異なる navi 固有の表現要件（`docs/glossary.md`）。
// app シェルと playground の両方がここを単一の真実として使う（issue c4b1e0）。
// `?raw` 等の Vite 依存はここには持ち込まない（純粋に保ち headless テスト可能にする）。

export type AgentState =
  | "thinking"
  | "running"
  | "editing"
  | "waiting_approval"
  | "done"
  | "error";

export type SourceId = "claude-code" | "codex" | "copilot" | "cursor";

/** スタックの可視性ポリシー（表示モード）。 */
export type DisplayMode = "show" | "hide" | "auto";

/**
 * 1 枚のステータスカードを描くのに必要な最小データ（props in）。
 * source は未知の文字列も許容する（バッジは未知ソースをフォールバック表示）。
 */
export interface StatusCardData {
  source: string;
  state: AgentState;
  /** 見出し（エージェント名やセッション名）。 */
  label: string;
  /** 任意の本文。空/未指定なら state ラベルにフォールバック。 */
  message?: string;
  /** 表示用プロジェクト名（無ければ cwd の末尾を使う）。 */
  project?: string;
  cwd?: string;
}

export const stateLabels: Record<AgentState, string> = {
  thinking: "Thinking",
  running: "Running",
  editing: "Editing",
  waiting_approval: "Waiting approval",
  done: "Ready",
  error: "Needs attention",
};

// 複数セッションが同時に動くとき、数字が大きい state が pet アニメを勝ち取る。
export const STATE_PRIORITY: Record<AgentState, number> = {
  error: 6,
  waiting_approval: 5,
  thinking: 4,
  running: 3,
  editing: 2,
  done: 1,
};

export const agentStates: AgentState[] = [
  "done",
  "thinking",
  "running",
  "editing",
  "waiting_approval",
  "error",
];

/**
 * 全セッションの state を STATE_PRIORITY で比較した最大値。pet アニメを決める。
 * セッションが無ければ done。
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

/** カード本文: 明示メッセージ（空白のみは無視）。無ければ state ラベル、最後に label。 */
export function cardMessage(data: StatusCardData): string {
  return data.message?.trim() ? data.message : stateLabels[data.state] ?? data.label;
}

/** ディレクトリ表示: プロジェクト名優先、無ければ cwd の末尾セグメント。 */
export function cardDir(data: Pick<StatusCardData, "project" | "cwd">): string | null {
  return (
    data.project ??
    (data.cwd ? data.cwd.split("/").filter(Boolean).at(-1) ?? null : null)
  );
}

/** "auto" 表示モードでは、最優先が done 以外のときにカードを表示する。 */
export function isVisibleInAuto(highest: AgentState): boolean {
  return highest !== "done";
}
