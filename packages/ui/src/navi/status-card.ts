// ステータスカード（1 セッションの状態表示）の DOM 部品。
//
// 「props in / DOM out」のダム部品。セッション管理・ドラッグ等のシェル配線は
// consumer（app シェル / playground）側に残す（issue c4b1e0）。caller が返値を
// 任意の親へ append する（このモジュールは append しない）。

import { type StatusCardData, type SourceId, cardMessage, cardDir } from "./state";
import { sourceConfig } from "./source-badge";

export interface StatusCardCallbacks {
  /** 閉じるボタン押下。引数は createStatusCard に渡した id。 */
  onClose: (id: string) => void;
}

export function createStatusCard(
  id: string,
  data: StatusCardData,
  cb: StatusCardCallbacks,
): HTMLElement {
  const card = document.createElement("div");
  card.className = "status-card";
  card.dataset.id = id;
  card.innerHTML = `
    <button class="status-card-close" type="button" aria-label="Remove session">×</button>
    <div class="source-badge" aria-label="Source agent"></div>
    <p class="status-card-title"></p>
    <p class="message"></p>
    <p class="cwd-label" hidden></p>
  `;
  card
    .querySelector<HTMLButtonElement>(".status-card-close")
    ?.addEventListener("click", (e) => {
      e.stopPropagation();
      cb.onClose(id);
    });
  updateStatusCard(card, data);
  return card;
}

export function updateStatusCard(card: HTMLElement, data: StatusCardData): void {
  card.dataset.state = data.state;
  card.dataset.source = data.source;

  const title = card.querySelector<HTMLParagraphElement>(".status-card-title");
  const msg = card.querySelector<HTMLParagraphElement>(".message");
  const cwdLabel = card.querySelector<HTMLParagraphElement>(".cwd-label");
  const sourceBadge = card.querySelector<HTMLDivElement>(".source-badge");

  if (title) title.textContent = data.label;
  if (msg) msg.textContent = cardMessage(data);

  const dir = cardDir(data);
  if (cwdLabel) {
    cwdLabel.textContent = dir ?? "";
    cwdLabel.hidden = !dir;
  }

  const cfg = sourceConfig[data.source as SourceId];
  if (sourceBadge) {
    if (cfg) {
      sourceBadge.innerHTML = cfg.svg;
      sourceBadge.dataset.source = data.source;
      sourceBadge.title = cfg.label;
    } else {
      // 未知ソース: ロゴ無し・ソース名をツールチップに
      sourceBadge.innerHTML = "";
      delete sourceBadge.dataset.source;
      sourceBadge.title = data.source;
    }
  }
}
