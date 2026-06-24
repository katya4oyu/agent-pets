// navi 固有 UI（吹き出しスタック・ソースバッジ・トグル/セッションカウント）の
// playground ローカル実装。
//
// codex-pet（スプライト描画 = `@navi/ui`）とは出自が異なる navi 固有の表現要件
// （`docs/concept.md` の「ui」責務）。ここでは `@navi/ui` への抽出（issue c4b1e0）は
// せず、playground のサンドボックス内で「props in / DOM out」のダム部品として組む。
// ここで詰めた構造・既定値を、後で `@navi/ui` のコンポーネント既定値や app シェルへ
// 焼き込む（issue d9a2f7 のワークフロー）。

export type AgentState =
  | "thinking"
  | "running"
  | "editing"
  | "waiting_approval"
  | "done"
  | "error";

export type SourceId = "claude-code" | "codex" | "copilot";

export interface SessionData {
  id: string;
  source: SourceId;
  state: AgentState;
  /** 吹き出しの見出し（エージェント名やセッション名）。 */
  label: string;
  /** 任意の本文。無ければ state ラベルにフォールバック。 */
  message?: string;
  /** 表示用プロジェクト名（無ければ cwd の末尾を使う）。 */
  project?: string;
  cwd?: string;
}

// ── pure helpers（app/src/state.ts のミラー。playground を自己完結に保つ） ──

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

export function highestPriorityState(states: Iterable<AgentState>): AgentState {
  let best: AgentState = "done";
  for (const state of states) {
    if (STATE_PRIORITY[state] > STATE_PRIORITY[best]) {
      best = state;
    }
  }
  return best;
}

export function bubbleMessage(session: SessionData): string {
  return session.message?.trim() ? session.message : stateLabels[session.state];
}

export function bubbleDir(session: SessionData): string | null {
  return (
    session.project ??
    (session.cwd ? session.cwd.split("/").filter(Boolean).at(-1) ?? null : null)
  );
}

export function isSpeechVisibleInAuto(highest: AgentState): boolean {
  return highest !== "done";
}

// ── ソースバッジ（claude-code / codex / copilot） ──
//
// app では `@lobehub/icons-static-svg` の実ロゴを使うが、playground は依存を増やさず
// currentColor で色付けする軽量グリフで代替する（色はチューニングで上書き可能）。

export interface SourceConfig {
  label: string;
  /** 既定色。playground のカラーピッカーで上書きされる。 */
  color: string;
  svg: string;
}

export const sourceConfig: Record<SourceId, SourceConfig> = {
  "claude-code": {
    label: "Claude Code",
    color: "#CC785C",
    // Anthropic のサンバースト風（4方向の星）。
    svg: `<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 2.2l1.9 6.4 6.4 1.9-6.4 1.9L12 18.8l-1.9-6.4L3.7 10.5l6.4-1.9z"/></svg>`,
  },
  codex: {
    label: "Codex",
    color: "#10A37F",
    // コード由来の `< >` シェブロン。
    svg: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 7l-4.5 5L9 17M15 7l4.5 5L15 17"/></svg>`,
  },
  copilot: {
    label: "Copilot",
    color: "#6F42C1",
    // ゴーグル（2つの目）風。
    svg: `<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M3 11.5c0-1.4 1.1-2.5 2.5-2.5h13c1.4 0 2.5 1.1 2.5 2.5v1c0 2.5-2 4.5-4.5 4.5-1.6 0-3-.8-3.8-2.1h-.4A4.5 4.5 0 0 1 8.5 17 4.5 4.5 0 0 1 3 12.5zm5.5.5a1.3 1.3 0 1 0 0 2.6 1.3 1.3 0 0 0 0-2.6m7 0a1.3 1.3 0 1 0 0 2.6 1.3 1.3 0 0 0 0-2.6"/></svg>`,
  },
};

// ── 吹き出し DOM（app/src/main.ts の createBubbleElement と同構造） ──

export interface BubbleCallbacks {
  onClose: (id: string) => void;
}

export function createBubble(session: SessionData, cb: BubbleCallbacks): HTMLElement {
  const bubble = document.createElement("div");
  bubble.className = "speech";
  bubble.dataset.id = session.id;
  bubble.innerHTML = `
    <button class="speech-close" type="button" aria-label="Remove session">×</button>
    <div class="source-badge" aria-label="Source agent"></div>
    <p class="speech-title"></p>
    <p class="message"></p>
    <p class="cwd-label" hidden></p>
    <span class="speech-tail" aria-hidden="true"></span>
  `;
  bubble
    .querySelector<HTMLButtonElement>(".speech-close")
    ?.addEventListener("click", (e) => {
      e.stopPropagation();
      cb.onClose(session.id);
    });
  updateBubble(bubble, session);
  return bubble;
}

export function updateBubble(bubble: HTMLElement, session: SessionData): void {
  bubble.dataset.state = session.state;
  bubble.dataset.source = session.source;

  const title = bubble.querySelector<HTMLParagraphElement>(".speech-title");
  const msg = bubble.querySelector<HTMLParagraphElement>(".message");
  const cwdLabel = bubble.querySelector<HTMLParagraphElement>(".cwd-label");
  const sourceBadge = bubble.querySelector<HTMLDivElement>(".source-badge");

  if (title) title.textContent = session.label;
  if (msg) msg.textContent = bubbleMessage(session);

  const dir = bubbleDir(session);
  if (cwdLabel) {
    cwdLabel.textContent = dir ?? "";
    cwdLabel.hidden = !dir;
  }

  const cfg = sourceConfig[session.source];
  if (sourceBadge) {
    sourceBadge.innerHTML = cfg.svg;
    sourceBadge.dataset.source = session.source;
    sourceBadge.title = cfg.label;
  }
}
