# 2026-06-14 Chat Summary Reply File Write Guard

## 背景

用户截图确认：模型声称已经续写并新建第2集文件，但实际写入文件的是对话区的总结回复，不是第2集正文。

## 根因

上一轮为了修复“缺少 fileOperations 时无法新建文件”，引入了本地 `writeFile` fallback，但判断过宽。模型回复中只要剥掉“已新建/已保存”后剩余内容超过长度门槛，就会被当成新文件正文。截图里的“本集承接...重点推进...文风和格式...”属于对话说明，不是独立文稿正文，却被保存进了文件。

## 变更

- 后端 `reply_can_seed_local_file_operation` 增加独立文稿形态门槛：只有回复剩余内容以 Markdown 标题、集标题或 fenced 内容开头时，才允许交给本地写文件 fallback。
- 前端 `createLocalWriteFileOperationFallback` 使用同样门槛，避免把总结型对话回复写入文件。
- 共创单测新增 `summary_style_reply_cannot_seed_local_write_tool_fallback`，覆盖截图中“已续写 + 重点推进 + 风格说明”的误存模式。
- 真实 exe E2E 新增 `testSummaryStyleReplyDoesNotCreateWrongEpisode`，验证总结型回复不会创建错误的 `第3集.md`，当前稿件也不被改动。

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：通过，44 个共创测试全绿。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml e2e --lib`：通过。
- MSVC 环境下 `npm run tauri -- build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\e2e-launch.ps1 -DebugPort 9222 -StopExisting` 后执行 `node scripts\e2e-smoke.mjs`：通过。覆盖对话驱动作品库/知识库增改删、明确正文 fallback 新建第2集、总结型回复不创建错误文件、Markdown 表格、划词添加到对话并发送、正文 inline diff。

## 产物

- 安装包：`release/Wridian-0.0.9-x64-setup.exe`

## 回退

- 回退 `src-tauri/src/cocreation.rs` 中 `looks_like_standalone_document_body` 门槛和对应测试。
- 回退 `src/chat/chatManager.ts` 中 `looksLikeStandaloneDocumentBody` 门槛。
- 回退 `scripts/e2e-smoke.mjs` 中总结型回复反例。
