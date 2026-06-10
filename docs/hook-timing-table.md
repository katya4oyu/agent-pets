# Hook タイミング一覧

## 凡例
- ✅ 対応済み（現在の実装に含まれる）
- ⬜ 未対応（イベントは届くが無視 or 未登録）

---

## フェーズ別タイミング表

| タイミング | Claude Code | Codex | Copilot | 現在の状態 | 備考 |
|---|---|---|---|---|---|
| **セッション開始** | `SessionStart` | `SessionStart` | `sessionStart` / `SessionStart` | `done` → Ready | resumeも含む |
| **プロンプト送信** | `UserPromptSubmit` | `UserPromptSubmit` | `userPromptSubmitted` / `UserPromptSubmit` | `thinking` → Thinking | promptフィールドあり |
| **ツール実行前（Shell系）** | `PreToolUse` (Bash) | `PreToolUse` (Bash) | `preToolUse` / `PreToolUse` (bash) | `running` → Running shell | command取得可 |
| **ツール実行前（Edit系）** | `PreToolUse` (Write/Edit) | `PreToolUse` (apply_patch) | `preToolUse` / `PreToolUse` (create/edit) | `editing` → Editing | file_path取得可 |
| **ツール実行前（その他）** | `PreToolUse` (Glob/Grep/Web等) | `PreToolUse` (MCP等) | `preToolUse` / `PreToolUse` (glob/grep等) | `running` → Using tool | ツール名のみ |
| **権限確認待ち** | `PermissionRequest` | `PermissionRequest` | `permissionRequest` / `PermissionRequest` | `waiting_approval` → Waiting approval | — |
| **権限拒否** | `PermissionDenied` | ⬜ なし | ⬜ なし | ⬜ 未対応 | Claude Code のみ |
| **ツール完了後** | `PostToolUse` | `PostToolUse` | `postToolUse` / `PostToolUse` | `running` → Tool completed | 結果取得可 |
| **ツール失敗** | `PostToolUseFailure` | ⬜ なし | `postToolUseFailure` / `PostToolUseFailure` | `error` → Tool failed | Codexにはない |
| **エラー発生** | `StopFailure` | ⬜ なし | `errorOccurred` / `ErrorOccurred` | `error` → Tool failed | 回復可否フラグあり |
| **通知・注意喚起** | `Notification` | ⬜ なし | `notification` / `Notification` | `waiting_approval` → Needs attention | アイドル・権限prompt等 |
| **応答完了・停止** | `Stop` | `Stop` | `agentStop` / `Stop` | `done` → Done | — |
| **セッション終了** | `SessionEnd` | ⬜ なし | `sessionEnd` / `SessionEnd` | ⬜ 未対応 | — |
| **サブエージェント開始** | `SubagentStart` | ⬜ なし | `subagentStart` | ⬜ 未対応 | Claude Code並列実行時 |
| **サブエージェント完了** | `SubagentStop` | ⬜ なし | `subagentStop` / `SubagentStop` | ⬜ 未対応 | — |
| **コンパクション開始** | `PreCompact` | ⬜ なし | `preCompact` / `PreCompact` | ⬜ 未対応 | 長いコンテキスト圧縮時 |
| **Elicitation（MCP確認）** | `Elicitation` | ⬜ なし | ⬜ なし | `waiting_approval` | Claude Code + MCP のみ |
| **CWD変更** | `CwdChanged` | ⬜ なし | ⬜ なし | ⬜ 未対応 | ディレクトリ移動時 |

---

## 各エージェントのフィールド差異（ツール関連）

| 項目 | Claude Code | Codex | Copilot (PascalCase) | Copilot (camelCase) |
|---|---|---|---|---|
| イベント名フィールド | `hook_event_name` | `hook_event_name` | `hook_event_name` | ❌ なし（形から推定） |
| ツール名 | `tool_name` | `tool_name` | `tool_name` | `toolName` |
| ツール引数 | `tool_input` | `tool_input` | `tool_input` | `toolArgs` |
| コマンド内容 | `tool_input.command` | `tool_input.command` | `tool_input.command` | `toolArgs.command` |
| ファイルパス | `tool_input.file_path` | `tool_input.file_path` | `tool_input.file_path` | `toolArgs.filePath` |
| セッションID | `session_id` | `session_id` | `session_id` | `sessionId` |
