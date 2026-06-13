# 对话文件操作工具块

## 背景

用户反馈通过右侧对话修改正文、以及在作品库/知识库文件树里新增、改名、删除一直不稳定。对照 `obsidian-copilot`、`claudian` 和 `CodePilot` 后，稳定模式不是把文件操作混在普通助手回复里，而是让文件写入/编辑作为独立工具结果展示，并保留正常回复。

## 变更

- 助手消息新增结构化 `fileOperations` 字段，用于保存本轮后端实际执行的文件树操作结果。
- 右侧聊天气泡保留模型正常 `reply`，文件写入、创建文件夹、重命名、移到回收站结果在气泡下方独立渲染为工具结果块。
- 后端不再用“已处理/未能处理”覆盖模型 `reply`；执行结果仍通过 `fileOperations` 返回，成功后继续刷新文件树，审计 JSONL 保持不变。
- 现有当前文件夹路由、已有文件 `writeFile` 拒绝、当前文件写入转 inline diff 逻辑不变。

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：通过，39 个 cocreation 测试。

## 回退

回退 `src/chat/messageRepository.ts`、`src/chat/chatManager.ts`、`src/chat/ChatPanel.tsx`、`src/App.css` 和 `src-tauri/src/cocreation.rs` 中本轮改动即可恢复旧行为；已执行的文件操作审计仍在运行目录 JSONL 中保留。
