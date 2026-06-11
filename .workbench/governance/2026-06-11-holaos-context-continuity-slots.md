# 2026-06-11 holaOS Context Continuity Slots

## 目标

按 holaOS continuity 分层思路收口 Wridian 对话上下文编译：每类上下文有独立槽位、预算、裁剪状态，并在发送后的用户消息上提供可查看的加载状态。

## 现场依据

- 项目地图已规定对话上下文采用固定槽位和预算，且不把作品记忆、知识卡和 skill 协议混写。
- 现有 `cocreation.rs` 已有预算常量和基础分槽，但前端没有拿到每轮加载状态，显式知识卡与相关稿件仍共用非 tool 槽。
- holaOS 只作为 continuity 和上下文边界参考；本轮不引入作品元素或 World Info。

## 变更

- 后端 prompt 编译拆成八个状态槽：当前稿件/选区、Project Mode、当前现场、compressed memory、显式知识卡、Relevant Notes、skill 协议、用户请求。
- 每个槽位返回 `loaded/itemCount/includedChars/budgetChars/truncated/note`，超预算时在对应槽位内裁剪。
- 前端把加载状态绑定到发送瞬间的用户消息，用折叠行显示“上下文 n/8”，展开后查看各槽位状态，不在对话区常驻刷屏。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib` 通过，13 个 cocreation 单测通过。
- `Invoke-WebRequest http://127.0.0.1:5173/` 返回 200；当前端口已有本地服务，未另启新服务。

## 回退

撤回 `src-tauri/src/cocreation.rs` 的 `ContextLoadStatus` 与槽位构造改动、前端 `contextLoadStatus` 类型/展示和 `src/App.css` 的 `.message-context-status` 样式即可恢复旧行为。
