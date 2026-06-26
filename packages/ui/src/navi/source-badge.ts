// ソースバッジ（claude-code / codex / copilot）。
//
// `@lobehub/icons-static-svg` の実ブランドロゴを `?raw` で取り込む。グリフは 2 系統:
//  - currentColor グリフ（単色塗り）… mono / tint で使用
//  - 公式配色グリフ … brand で使用（codex は白タイル＋グラデ、claude はクレイ）
// 表示バリアントは 3 つ:
//  - mono : モノクロ。全ソース共通の中立グレーで着色（--badge-mono）。
//  - tint : 単色。ソースごとのブランド色で着色（--src-*）。
//  - brand: 公式配色そのまま。再現度優先。
// copilot は **GitHub Copilot のロボットヘッド**（公式は単色のみ。多色版は存在しない）。

import type { SourceId } from "./state";

import claudeCodeSvg from "@lobehub/icons-static-svg/icons/claudecode.svg?raw";
import claudeCodeColorSvg from "@lobehub/icons-static-svg/icons/claudecode-color.svg?raw";
import codexSvg from "@lobehub/icons-static-svg/icons/codex.svg?raw";
import codexColorSvg from "@lobehub/icons-static-svg/icons/codex-color.svg?raw";
import githubCopilotSvg from "@lobehub/icons-static-svg/icons/githubcopilot.svg?raw";

/** バッジのアイコン配色。mono=モノクロ（共通グレー）/ tint=単色（ブランド色）/ brand=公式配色。 */
export type BadgeVariant = "mono" | "tint" | "brand";

export interface SourceConfig {
  label: string;
  /** 既定のブランド色（tint 時の着色）。consumer 側で --src-* により上書き可能。 */
  color: string;
  /** currentColor グリフ（mono / tint 用）。 */
  svg: string;
  /** brand グリフ（公式配色）。多色版が無いソースは svg と同一。 */
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

