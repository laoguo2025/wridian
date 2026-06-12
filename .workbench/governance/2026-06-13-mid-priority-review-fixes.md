# 2026-06-13 中优先级审查修复记录

## 范围

- 修复模型账户状态读取触发 OAuth 刷新的副作用，并把异步链路里的同步凭据刷新移到阻塞线程。
- 外部模型/OAuth 错误响应统一走脱敏摘要，避免 token、key、secret 等字段进入界面错误文本。
- 协作模型输出只接受有效 JSON 或可提取的 fenced/balanced JSON，不再把纯文本当成成功结构化响应。
- 知识库 hot、fold、体检报告、manifest 和低风险修复文件写入前检查父级与目标不能是符号链接或 Windows 重解析点。
- 移除未使用 Tauri 命令注册及对应孤岛 API 壳；移除 NSIS 卸载 hook 里的硬编码整库删除 fallback。
- 前端打开记忆目录改为语义化后端命令，模型设置里补上已连接服务编辑入口。

## 验证

- `npx tsc --noEmit`：通过。
- `npm run build`：通过。
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`：通过。
- `cmd.exe /d /s /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul && cargo test --manifest-path src-tauri/Cargo.toml'`：113 个测试通过。

## 回退

- 如需回退本轮变更，回退对应提交即可；本轮未改写外部状态，未 push。
