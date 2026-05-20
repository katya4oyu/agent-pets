# Research Notes

## Desktop framework

Tauri v2 is the recommended first choice.

Reasons:

- Lightweight enough for a resident companion.
- Native tray support.
- Notification plugin support.
- Rust backend is a good fit for a localhost event receiver.
- Transparent floating windows are achievable.

Electron is a safe fallback when deep Node or VS Code integration becomes more
important than footprint. Electrobun is interesting but less proven. MoonBit is
not a good primary app framework for this yet, but could be used later for
WASM-based rule modules.

## References

- Codex Hooks: https://developers.openai.com/codex/hooks
- Codex app settings and pets: https://developers.openai.com/codex/app/settings
- Tauri tray: https://v2.tauri.app/learn/system-tray/
- Tauri notifications: https://v2.tauri.app/plugin/notification/
- Electron tray: https://www.electronjs.org/docs/latest/api/tray
- Electrobun tray: https://blackboard.sh/electrobun/docs/apis/tray/
- Claude Code hooks: https://code.claude.com/docs/en/hooks
- GitHub Copilot CLI hooks: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/use-hooks
