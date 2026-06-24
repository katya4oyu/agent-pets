import { defineConfig, type Plugin } from "vite";
import { fileURLToPath } from "node:url";
import { rename } from "node:fs/promises";
import { join } from "node:path";

const htmlEntry = (file: string) => fileURLToPath(new URL(`./${file}`, import.meta.url));

// 公開（Cloudflare）ビルド `--mode playground` では playground.html を
// `/`（index.html）として出力する。Vite は HTML エントリの出力名を元ファイル名から
// 決めるため、書き出し後に dist 内で playground.html を index.html へリネームする。
// （bundle オブジェクトの書き換えは Rolldown=Vite8 で禁止されているため writeBundle で実施。）
function playgroundAsIndex(): Plugin {
  return {
    name: "navi-playground-as-index",
    enforce: "post",
    async writeBundle(options) {
      const dir = options.dir ?? "dist";
      await rename(join(dir, "playground.html"), join(dir, "index.html"));
    },
  };
}

// 本番（Tauri）= index.html / 開発用 = playground.html の 2 エントリ。
// dev では両方サーブされ、playground は /playground.html で開ける。
// `--mode playground`（pnpm run build:playground）では playground だけを `/` に出力する。
export default defineConfig(({ mode }) => {
  const playgroundOnly = mode === "playground";

  return {
    // Tauri の dev サーバーを邪魔しないよう Vite のログは既定のまま。
    clearScreen: false,
    plugins: playgroundOnly ? [playgroundAsIndex()] : [],
    build: {
      rollupOptions: {
        input: playgroundOnly
          ? { playground: htmlEntry("playground.html") }
          : {
              main: htmlEntry("index.html"),
              playground: htmlEntry("playground.html"),
            },
      },
    },
  };
});
