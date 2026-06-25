// Vite の `?raw` インポート（ソースバッジの SVG）を tsc で解決するためのアンビエント宣言。
declare module "*.svg?raw" {
  const content: string;
  export default content;
}
