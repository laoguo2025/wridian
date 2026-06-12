# Gemini OAuth Cloud Code Assist 修正

## 背景

截图中的 Gemini OAuth 账号直连仍显示 `https://generativelanguage.googleapis.com/v1beta`，后端也把 OAuth access token 直接用于 Gemini API `models/<model>:generateContent`。对照本地 `hermes-agent` 源码后确认该链路不对：`gemini` API Key 版走 `generativelanguage.googleapis.com`，`google-gemini-cli` OAuth 版走 Gemini CLI / Code Assist 后端。

## 变更

- Gemini OAuth provider 的 Base URL 改为内部 marker `cloudcode-pa://google`。
- Google OAuth client 默认使用 Gemini CLI 同款公共桌面 OAuth client；`WRIDIAN_GOOGLE_OAUTH_CLIENT_ID` / `WRIDIAN_GOOGLE_OAUTH_CLIENT_SECRET` 只作为高级环境变量覆盖项。
- 登录、测试和对话请求会解析并保存 Code Assist project。
- OAuth 版 Gemini 请求改为 `https://cloudcode-pa.googleapis.com/v1internal:generateContent`，请求体包装 `project/model/user_prompt_id/request`。
- API Key 版 Gemini 保持原有 `generativelanguage.googleapis.com/v1beta` 路径。

## 回退

如 Code Assist 后端策略变化导致 OAuth 版不可用，可回退本次对 `src-tauri/src/model_accounts.rs`、`src-tauri/src/cocreation.rs`、`src/settings/providerCatalog.ts` 的修改；API Key 版 Gemini 不受影响。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml model_accounts --lib` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib` 通过。
