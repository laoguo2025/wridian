# 2026-06-11 Chat Continuity Session Fork

## 目标

实现写作对话“续接”最小闭环：用户说继续、回到那段或按刚才方向改时，Wridian 能恢复当前现场、上轮意图、判断和下一步建议，并支持从助手回复分叉新方向。

## 现场依据

- 现有后端已读取 `.wridian/active-context.json` 进入“当前现场”槽位，但此前没有写入者。
- 现有聊天记录只保存 `.wridian/chat/<session>.md`，适合归档，不适合恢复消息树或记录 fork 元数据。
- 项目地图已把 Pi 的 session tree/continue/fork 和 holaOS 的 runtime continuity 分层列为后续参考，本轮只借分层和树状会话思路，不引入外部系统。

## 变更

- `wridian_save_chat_transcript` 同步写入 Markdown 归档、`active-context.json`、最近活动 session index、session JSON、session history JSONL 和 compact summary 交接卡。
- 前端启动时调用 `wridian_load_chat_continuity` 恢复最近活动会话消息。
- 每轮发送后根据当前作品、选区/稿件片段、用户输入、助手回复生成 active context。
- 助手消息增加“分叉”动作，从该回复截断出新 session，并记录 parent session 与 forked message。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml chat_persistence --lib` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml` 通过。

## 回退

- 前端回退：撤回 `src/chat/chatManager.ts` 的恢复/fork/active context 写入、`src/chat/ChatPanel.tsx` 的分叉按钮、`src/App.tsx` 的分叉处理。
- 后端回退：撤回 `src-tauri/src/chat_persistence.rs` 的 session index/session JSON/history/active-context 写入与 `wridian_load_chat_continuity` 命令，保留旧 Markdown transcript 即可恢复原行为。
