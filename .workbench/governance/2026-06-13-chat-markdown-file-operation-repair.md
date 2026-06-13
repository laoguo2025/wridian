# 2026-06-13 Chat Markdown And File Operation Repair

## 背景

用户截图反馈右侧对话中的 Markdown 标题和加粗符号直接暴露给用户；同时通过对话要求在作品库/知识库文件树中新建、修改或删除文件时，模型仍可能只返回正文，不返回可执行 `fileOperations`。

## 变更

- 右侧助手消息改为本地 Markdown 预览渲染，支持标题、段落、列表、加粗、行内代码、代码块、链接和基础表格。
- 表格包裹在气泡内部横向滚动容器中，随右侧气泡宽度变化，不撑破对话栏。
- 用户消息保持按原文展示，避免编辑/复制语义变化。
- 后端文件树操作补救条件从“用户要求写文件且回复声称已写入”扩展为“用户明确要求文件树变更但 `fileOperations` 为空”。
- 补救覆盖新建、写入、保存、重命名、删除/回收站、创建目录；补救仍失败时拦截普通回复，明确说明没有执行任何文件树操作。
- 新增单测覆盖“新建一个文档，续写第2集”但模型只返回 Markdown 正文的场景。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib` 通过，40 个共创相关测试全绿。

## 回退

- 前端回退 `src/chat/ChatPanel.tsx` 的 `MarkdownMessage` 使用点和相关 Markdown CSS。
- 后端回退 `src-tauri/src/cocreation.rs` 中 `should_repair_missing_file_operations` 的触发条件、补救提示和新增测试。
