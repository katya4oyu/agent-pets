---
id: 3d107c
title: プロダクト名を "navi" として agent-pets 内に実装する
type: chore
status: done
priority: high
---

リポジトリ名は `agent-pets` のまま（PET の中にネットナビが宿る世界観に沿う）。
ユーザーが目にするプロダクト名・表示名を "navi" にする。

## 変更箇所

- [x] `tauri.conf.json` の `productName` → "navi"
- [x] `tauri.conf.json` の `identifier` → `com.katya4oyu.navi`
- [x] アプリウィンドウタイトル・トレイアイコンのツールチップ
- [x] `~/.agent-pets/` → `~/.navi/`（ポートファイル、設定ディレクトリ）
- [x] バイナリ名 `agent_pets_hook` → `navi-hook`
- [x] `setup_hooks` / `setup_codex` のパス文字列
- [x] README の "Agent Pets" → "navi" への表記更新

## 備考

- GitHub リポジトリ名 `agent-pets` はそのまま維持
- Codex Pets 派生であることは README に明記する
