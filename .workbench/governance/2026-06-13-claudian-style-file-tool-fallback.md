# 2026-06-13 Claudian Style File Tool Fallback

## 背景

用户反馈对话要求“根据第1集剧情，续写第2集，在作品库中新建个文档”时，Wridian 仍提示模型没有返回可执行文件树操作，没有真正新建文件。对比 Claudian 后确认其关键链路是显式工具流：消息保存 `Read/Edit/Write` 等工具调用，写改工具单独渲染并展示结果，而不是依赖普通回复文案。

## 参考

- `YishenTu/claudian` commit `d2a6684`。
- 关键参考点：
  - `src/core/tools/toolNames.ts`：显式区分 `Read/Write/Edit` 等工具。
  - `src/features/chat/controllers/StreamController.ts`：工具调用进入 `toolCalls` 和 pending tool state，Write/Edit 走专门渲染。
  - `src/features/chat/rendering/WriteEditRenderer.ts`：Write/Edit 工具块独立展示状态、路径和 diff/完成结果。
  - `src/core/storage/VaultFileAdapter.ts`：写入前确保父目录存在，文件操作通过受限 adapter 执行。

## 变更

- 后端新增 `wridian_apply_chat_file_operations`，复用现有 `writeFile/createFolder/rename/trash` 安全执行、库根边界校验和审计。
- 该命令支持传入当前打开文件路径；新建作品文件只给文件名时，后端按当前作品文件所在文件夹路由。
- 后端补救失败时，如果模型回复是可写入新文档的实质正文，不再替换成失败提示；仍保留对“声称已新建/已写入”的假成功拦截。
- 前端在模型漏 `fileOperations` 且用户明确要求新建/写入文档时，基于模型正文合成本地 `writeFile` 工具操作，调用后端命令执行，并把执行结果挂到同一助手消息的工具结果块。
- 成功执行后继续刷新文件树。
- 本地工具 fallback 推断文件名时，优先识别“续写第 N 集”为 `第N集.md`；只有“命名为/文件名为”或“新建《某某》文档”这类明确目标名才取书名号内容，避免把作品名误当新文件名。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib` 通过，42 个共创相关测试全绿。

## 回退

- 前端回退 `src/chat/chatManager.ts` 中本地 `writeFile` fallback 和 `src/chat/cocreationClient.ts` 的 `applyChatFileOperations`。
- 后端回退 `wridian_apply_chat_file_operations` 命令、命令注册，以及 `reply_can_seed_local_file_operation` 分支。
