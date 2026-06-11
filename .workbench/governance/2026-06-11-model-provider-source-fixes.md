# 2026-06-11 模型供应商源码对齐修复

## 目标

用户反馈多个模型对接不可用，要求对照 `NousResearch/hermes-agent` 和 `op7418/CodePilot` 源码修复 Wridian 的模型接入。

## 排查依据

- Hermes provider 实现显示 DeepSeek OpenAI-compatible V4 系列需要显式 thinking 控制，Kimi/Moonshot 的 thinking 与 `reasoning_effort` 不能同时发送；官方 DeepSeek / Moonshot 文档确认 Wridian 裸 HTTP 请求体应发送顶层 `thinking`，不是 OpenAI SDK 的 `extra_body` 包装字段。
- Hermes Gemini native adapter 显示 Google AI Studio API Key 应直连原生 `generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`，用 `x-goog-api-key` 鉴权，并在未传 max tokens 时显式设置 `maxOutputTokens=65535`，避免 Google 原生接口采用过低默认输出上限。
- Hermes Google Gemini CLI adapter 显示 OAuth 不是普通 AI Studio 直连，而是 Cloud Code Assist 的 `cloudcode-pa.googleapis.com/v1internal:*` 包装协议；该通道与 Wridian 当前 Google OAuth 直连实现不同，需独立切换。
- Hermes MiniMax provider 显示 Anthropic Messages 端点为 `/anthropic`，不同区域 endpoint 独立。
- CodePilot provider catalog 显示 GLM/MiniMax/MiMo/Bailian/DeepSeek 这些 Claude Code 兼容预设依赖 role model / upstream model 映射，不能把 UI 别名当真实模型 ID 直发。
- CodePilot MiMo 回归测试说明 `model_names` 必须暴露并持久化，否则会回退到过期默认模型。

## 本轮变更

- Anthropic 官方预设默认模型从裸别名改为真实上游模型 ID，避免直连官方 API 时发送 `sonnet/opus/haiku`。
- 后端读取已保存模型时按 provider `extraEnv` 解析 `ANTHROPIC_DEFAULT_*_MODEL`，已保存别名也会转成真实模型。
- 连接测试透传 `extraEnv`，让保存前测试和保存后对话使用同一套模型解析。
- OpenAI-compatible 对话请求在 `response_format: json_object` 被兼容端点拒绝时，仅对参数类错误自动重试为 prompt-only JSON。
- DeepSeek OpenAI-compatible V4 / Reasoner 请求体补顶层 `thinking.type=disabled`，避免默认 thinking 引发后续 reasoning_content 回传错误。
- 用户后续要求删除具体厂商直连第三方 API 卡片，因此 Moonshot/Kimi OpenAI-compatible 直连专属请求体改动已移除；如需使用 Moonshot/Kimi 等普通 OpenAI 兼容服务，应走通用 `OpenAI-Compatible API` 手动配置。
- Gemini API Key 直连路径补 `maxOutputTokens=65535`，对齐 Hermes native adapter，降低 Gemini 原生 API 截断 Wridian JSON 回复的概率。
- 用户后续要求删除具体厂商直连第三方 API 卡片，因此 DeepSeek API、Moonshot API、Z.AI API、Xiaomi MiMo API、Alibaba Coding API、MiniMax API CN/Global 等独立直连预设已移除；第三方 API 区只保留通用 Anthropic / OpenAI-compatible 入口。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib` 通过，58 个测试全部通过。
- `git diff --check` 无空白错误；仅有 Windows 行尾提示。

## 回退

如本轮修复引发特定供应商不兼容，可回退本次提交，或只移除新增直连预设 / OpenAI-compatible provider extras；既有套餐型预设未被替换，回退影响面较低。
