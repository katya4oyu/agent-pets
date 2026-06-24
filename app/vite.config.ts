import { defineConfig, type Plugin } from "vite";
import { fileURLToPath } from "node:url";

const htmlEntry = (file: string) => fileURLToPath(new URL(`./${file}`, import.meta.url));

// 公開（Cloudflare）ビルド `--mode playground` では playground.html を
// `/`（index.html）として出力する。Vite は HTML エントリの出力名を元ファイル名から
// 決めるため、バンドル後に playground.html を index.html へ差し替えるプラグインで対応。
function playgroundAsIndex(): Plugin {
  return {
    name: "navi-playground-as-index",
    enforce: "post",
    generateBundle(_options, bundle) {
      const entry = bundle["playground.html"];
      if (entry) {
        delete bundle["playground.html"];
        entry.fileName = "index.html";
        bundle["index.html"] = entry;
      }
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
