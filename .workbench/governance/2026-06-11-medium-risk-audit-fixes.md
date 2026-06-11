# Medium Risk Audit Fixes

## 原因

继续修复审查中剩余的中风险问题：未知模型协议会落入 OpenAI-compatible fallback、Gemini API Key 被拼入 URL、本地 opener capability 暴露任意路径打开、模型切换失败后 UI 保持错误选择、Lexical pill 删除后外层状态可能残留、旧 custom API 命令仍暴露。

## 变更

- 模型协议运行时分发改为显式白名单；未知或旧别名协议不再默认按 OpenAI-compatible 发送。
- Gemini 对话和连接测试改用 `x-goog-api-key` header，不再把 API Key 拼到 URL query。
- Base URL 拒绝 query 和 fragment，避免把密钥或隐式参数塞进 endpoint。
- 新增后端受控本地打开命令，只允许打开当前作品库、默认作品库、知识库或 Wridian runtime 内路径；前端不再直接调用 opener `openPath`，Tauri capability 移除 `opener:allow-open-path`。
- 旧 custom API Tauri 命令从命令表和实现中移除；保留旧配置迁移读取逻辑。
- 模型选择失败时前端回滚到之前模型。
- Lexical pill 删除时立即同步外层 pill 状态，避免 stale pill 随发送进入上下文。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml`（48 passed）

## 回退

如需回退，可恢复本次涉及的模型协议分发、Gemini 请求认证方式、opener capability 和前端调用；但会重新暴露错误协议静默发送、URL 泄露 key、任意本地路径打开和 stale pill 上下文风险。
