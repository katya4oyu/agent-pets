import { defineConfig } from "vite";

// Tauri フロント（index.html → main.ts）の単一エントリ。
// 見た目デバッグ用の playground / gallery は examples/playground へ分離済み。
export default defineConfig({
  // Tauri の dev サーバーを邪魔しないよう Vite のログは既定のまま。
  clearScreen: false,
});
