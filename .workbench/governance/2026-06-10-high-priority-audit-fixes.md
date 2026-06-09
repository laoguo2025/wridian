# High Priority Audit Fixes

## 原因

修复审查发现的高优先级问题：文件切换保存竞态、模型 API 明文/非 TLS 风险、发给模型的本机绝对路径泄露、运行产物和本机 Cargo 配置误提交风险。

## 变更

- 前端打开文件期间使用 `loadingPath` 阻止保存链路把旧正文写入新路径，并在打开成功后更新文件上下文缓存。
- 自定义 API 配置只允许 HTTPS；`localhost` / `127.0.0.1` 作为本地调试例外。
- 自定义 API Key 改存 Windows Credential Manager，配置文件只保留 `keyStored` 状态；旧明文配置首次读取时迁移。
- 对话 prompt 发送给模型时只包含当前稿件文件名，不发送本机绝对路径。
- `.workbench/runtime/`、测试版 exe、`src-tauri/target/` 和 `.cargo/config.toml` 加入忽略。
- 新增 `scripts/cargo-msvc.ps1` 作为 PowerShell 下的 Rust/MSVC 检查入口，替代提交本机 `.cargo/config.toml`。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml`

## 回退

如需回退，可恢复 `.cargo/config.toml` 和旧模型账户写入逻辑；但会重新引入本机路径耦合和 API Key 明文落盘风险。
