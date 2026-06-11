# Anthropic 兼容服务响应修复

## 背景

用户在模型账户连接 DeepSeek 和 Xiaomi MiMo Token Plan 时遇到“Anthropic 响应中没有可用文本”。这些服务走 Anthropic 兼容协议，但返回格式不一定严格等同官方非流式 Messages JSON。

## 变更

- Anthropic 连接测试先发非流式请求；若 HTTP 成功但没有可用文本，自动以 `stream: true` 重试。
- 实际对话链路采用同样的非流式优先、流式兜底策略。
- Anthropic 响应解析扩展到 `content` 字符串、嵌套 `data/message/output_text`、OpenAI-compatible JSON、Anthropic SSE 和 OpenAI 风格 SSE。
- 第三方 API 继续只保留通用入口；国内 Claude Code 兼容套餐服务入口保留，统一复用 Anthropic 兼容链路。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib` 通过，61 项测试通过。

## 回退

回退本轮提交即可恢复原始非流式单路径解析；不会影响已保存 API Key，凭据仍在 Windows Credential Manager。
