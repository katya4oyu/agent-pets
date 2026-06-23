import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

// 本番（Tauri）= index.html / 開発用 = playground.html の 2 エントリ。
// dev では両方サーブされ、playground は /playground.html で開ける。
export default defineConfig({
  // Tauri の dev サーバーを邪魔しないよう Vite のログは既定のまま。
  clearScreen: false,
  build: {
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL("./index.html", import.meta.url)),
        playground: fileURLToPath(new URL("./playground.html", import.meta.url)),
      },
    },
  },
});
