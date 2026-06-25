// ソースバッジ（claude-code / codex / copilot）。
//
// `@lobehub/icons-static-svg` の実ブランドロゴを `?raw` で取り込む。各 SVG は
// `fill="currentColor"` なので、バッジ側の `color`（= app/playground の --src-* 変数や
// 既定色）で着色される。

import type { SourceId } from "./state";

import claudeCodeSvg from "@lobehub/icons-static-svg/icons/claudecode.svg?raw";
import codexSvg from "@lobehub/icons-static-svg/icons/codex.svg?raw";
import copilotSvg from "@lobehub/icons-static-svg/icons/copilot.svg?raw";

export interface SourceConfig {
  label: string;
  /** 既定のブランド色。consumer 側で --src-* により上書き可能。 */
  color: string;
  svg: string;
}

export const sourceConfig: Record<SourceId, SourceConfig> = {
  "claude-code": { label: "Claude Code", color: "#CC785C", svg: claudeCodeSvg },
  codex: { label: "Codex", color: "#10A37F", svg: codexSvg },
  copilot: { label: "Copilot", color: "#6F42C1", svg: copilotSvg },
};
