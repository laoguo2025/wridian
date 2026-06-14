# 2026-06-14 Chat Plain Response Routing

## 问题

用户在对话区发送普通问答 `列出第1集的人物关系图` 时，界面只保存/展示了很短的标题式回复。

## 根因

普通问答被共创 JSON 链路包了一层，模型在 `reply` 字段里很容易只吐出开头，Wridian 又把这段短文本当成最终回复保存。

## 处理

- 给普通问答增加纯文本回复链路，不再强制走 JSON 共创协议。
- 普通问答 prompt 改为 Markdown 预览友好输出，明确禁止文件树操作和正文修改。
- `列出第1集的人物关系图`、表格、清单、总结、解释类请求走纯文本回答。
- 文件树新建/修改/删除/重命名仍保留结构化链路。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`
- `npm run build`
- `npm run tauri -- build` in MSVC environment
- 真实启动 `src-tauri\target\release\wridian.exe`，发送 `列出第1集的人物关系图`
- 结果：UI 回复长度 533，session 内 assistant 回复长度 606，不再只剩标题

