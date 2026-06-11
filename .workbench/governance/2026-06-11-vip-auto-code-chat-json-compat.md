# 2026-06-11 VIP Auto-Code Chat JSON Compatibility

## 目标

- 使用用户提供的临时 API Key 验证 `https://vip.auto-code.net` 是否能承载 Wridian 对话请求。
- 不把 API Key 写入仓库、配置文件或长期文档。

## 排查结果

- `POST https://vip.auto-code.net/v1/chat/completions` 可用。
- `POST https://vip.auto-code.net/v1/responses` 和 `POST https://vip.auto-code.net/responses` 可用。
- 裸 `POST https://vip.auto-code.net/chat/completions` 返回 Auto-Code 网页 HTML，不适合作为 Wridian 对话 endpoint。
- Wridian 的 OpenAI-compatible 配置应填写 Base URL：`https://vip.auto-code.net`；后端会自动补 `/v1/chat/completions`。

## 发现的问题

- 该网关在 `response_format: { "type": "json_object" }` 下要求发送给模型的 user message 中包含小写 `json`。
- Wridian 原先只在 system prompt 使用大写 `JSON`，并且实际 user prompt 不含小写 `json`，因此真实对话路径会返回 400。
- 模型设置里的连接测试不带 `response_format`，所以会误判为“连接成功但正式对话失败”。

## 本轮修复

- 在系统提示中写明 `json object`。
- 在对话上下文编译出的用户消息中写明“必须返回 json object，字段为 reply、edits、memories”。
- 补单元测试确认编译后的对话 prompt 包含 `json object`。

## 验证

- 真实 API：`/v1/chat/completions` + `response_format: json_object` + Wridian 形态 prompt 返回 200。
- 返回内容可解析为 Wridian 需要的 `reply / edits / memories`，且 `edits[0].target` 精确命中稿件原文。
- 客户端 abort 模拟返回 `AbortError`，符合前端 Stop 放弃当前请求的预期。
- `npx tsc --noEmit` 通过。
- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib` 通过。
