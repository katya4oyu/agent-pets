import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

const htmlEntry = (file: string) => fileURLToPath(new URL(`./${file}`, import.meta.url));

// 独立した見た目デバッグ用アプリ。index.html = playground（Cloudflare 公開時の `/`）、
// gallery.html = アニメ一覧。Tauri は一切関与しない。
export default defineConfig({
  clearScreen: false,
  build: {
    rollupOptions: {
      input: {
        index: htmlEntry("index.html"),
        gallery: htmlEntry("gallery.html"),
      },
    },
  },
});
