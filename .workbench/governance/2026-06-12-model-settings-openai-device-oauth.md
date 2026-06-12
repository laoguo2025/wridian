# 2026-06-12 模型设置与 OpenAI device OAuth

## 背景

用户截图反馈：

- 模型配置二级弹窗周围有光晕。
- Gemini 默认模型列表需要改为 `gemini-3.5-flash`、`gemini-3.1-pro-preview`、`gemini-2.5-pro`。
- Anthropic 默认模型列表需要改为当前截图中的完整 Claude 模型 ID。
- OpenAI/ChatGPT/Codex 点击浏览器 OAuth 登录后，`auth.openai.com/log-in` 页面报 `Unexpected token '<'`。

## 排查

本地 Hermes `hermes_cli/auth.py` 的 OpenAI Codex 登录已经不走 `https://auth.openai.com/oauth/authorize`，而是：

1. `POST https://auth.openai.com/api/accounts/deviceauth/usercode` 获取 `device_auth_id` 和 `user_code`。
2. 用户打开 `https://auth.openai.com/codex/device` 输入验证码。
3. `POST https://auth.openai.com/api/accounts/deviceauth/token` 轮询 `authorization_code` 与 `code_verifier`。
4. 用 `https://auth.openai.com/oauth/token` 换取 Codex OAuth token，redirect URI 为 `https://auth.openai.com/deviceauth/callback`。

Wridian 原实现仍打开 `oauth/authorize` 并监听本地回调，和 Hermes 当前实现不一致，符合截图中 OpenAI 登录页前端报 HTML/JSON 解析错的表现。

## 变更

- OpenAI 登录拆为 `wridian_openai_oauth_start` / `wridian_openai_oauth_complete`，前端显示验证码并在用户完成网页登录后轮询换 token。
- OpenAI 帮助入口从 `oauth/authorize` 改为 `codex/device`。
- OAuth 登录后落库模型列表复用后端默认列表，避免弹窗默认和登录写回不一致。
- Gemini API Key 与 Gemini OAuth 默认模型统一为用户指定三项。
- Anthropic 官方 OAuth 默认模型改为完整 Claude 模型 ID 列表，旧 `opus` 别名同步指向 `claude-opus-4-8`。
- 二级连接弹窗去掉 `box-shadow` 光晕。

## 验证计划

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`
- `git diff --check`

## 回退

如 OpenAI device-code 上游不可用，可回退本次 OpenAI start/complete 命令与前端调用，恢复旧本地回调 authorize 流程；模型默认列表和弹窗样式可独立保留或单独回退。
