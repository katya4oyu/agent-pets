// ソースバッジ（claude-code / codex / copilot）。
//
// `@lobehub/icons-static-svg` の実ブランドロゴを `?raw` で取り込む。2 系統を持つ:
//  - mono : `fill="currentColor"` の単色グリフ。バッジの `color`（--src-* / 既定色）で着色。
//  - brand: 公式配色そのまま（codex は白タイル＋グラデ、claude はクレイ単色）。再現度優先。
// copilot は **GitHub Copilot のロボットヘッド**（公式は単色のみ。多色版は存在しない）。

import type { SourceId } from "./state";

import claudeCodeSvg from "@lobehub/icons-static-svg/icons/claudecode.svg?raw";
import claudeCodeColorSvg from "@lobehub/icons-static-svg/icons/claudecode-color.svg?raw";
import codexSvg from "@lobehub/icons-static-svg/icons/codex.svg?raw";
import codexColorSvg from "@lobehub/icons-static-svg/icons/codex-color.svg?raw";
import githubCopilotSvg from "@lobehub/icons-static-svg/icons/githubcopilot.svg?raw";

/** バッジのアイコン配色。mono=単色（ブランド色で着色）/ brand=公式配色。 */
export type BadgeVariant = "mono" | "brand";

export interface SourceConfig {
  label: string;
  /** 既定のブランド色（mono 時の着色）。consumer 側で --src-* により上書き可能。 */
  color: string;
  /** mono グリフ（currentColor）。 */
  svg: string;
  /** brand グリフ（公式配色）。多色版が無いソースは mono と同一。 */
  colorSvg: string;
}

export const sourceConfig: Record<SourceId, SourceConfig> = {
  "claude-code": {
    label: "Claude Code",
    color: "#CC785C",
    svg: claudeCodeSvg,
    colorSvg: claudeCodeColorSvg,
  },
  codex: {
    label: "Codex",
    color: "#10A37F",
    svg: codexSvg,
    colorSvg: codexColorSvg,
  },
  copilot: {
    label: "Copilot",
    color: "#6F42C1",
    svg: githubCopilotSvg,
    colorSvg: githubCopilotSvg, // GitHub Copilot は公式単色のみ
  },
};

/** variant に応じた SVG を返す。 */
export function sourceSvg(cfg: SourceConfig, variant: BadgeVariant): string {
  return variant === "brand" ? cfg.colorSvg : cfg.svg;
}

