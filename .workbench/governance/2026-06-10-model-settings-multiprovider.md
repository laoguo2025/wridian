# 2026-06-10 模型配置 CodePilot 复刻修正

## 目标

- 收掉上一版自研 provider/protocol/UI。
- 以 CodePilot provider catalog 和 Provider Manager 卡片布局为主要参照。
- 把 Hermes 的 Gemini OAuth 账号入口 `google-gemini-cli` / `gemini-oauth` 加入 Wridian。
- 已配置 provider 的模型继续在对话区模型列表中可切换。

## 变更

- 新增 `src/settings/providerCatalog.ts`，集中承载 CodePilot 风格的 `presetKey/protocol/authStyle/defaultModels/defaultEnvOverrides/meta`。
- `ModelSettingsDialog` 改为 CodePilot 式“已连接服务卡片 + 添加服务 preset + 连接弹窗”，弹窗尺寸继续与知识图谱一致。
- 根据 UI 复核要求，弹窗改为“已连接服务”上、“添加服务”下；删除顶部副标题、空状态小容器和弹窗内模型配置区。添加服务不再分页，按“授权登录 / 国内服务 / 第三方API”同页三列展示，已配置 provider 自动从添加区消失。
- 根据后续卡片复核要求，所有 provider 卡片移除左侧头像图标，右侧只保留“连接”或“断开”动作；已连接服务改为与添加服务相同的紧凑卡片，不再展示详情表、状态 pill 或底部接入类型标签。
- 根据 CodePilot 的 Aliyun Bailian 条目补入添加服务入口：Aliyun Bailian Coding Plan 与 Aliyun Bailian Token Plan 分别作为独立 provider 展示和配置，不互相替代。
- 根据所有供应商的 SDK base URL 形态修复请求 endpoint：Anthropic 兼容类统一从 catalog Base URL 补 `/v1/messages`，完整 `/v1/messages` 不重复拼接；OpenAI-compatible 统一补 `/v1/chat/completions`，完整 `/chat/completions` 不重复拼接。
- 用户界面移除 Hermes / CodePilot 字样；模型连接小弹窗增加“测试”按钮，测试当前表单配置，不要求先保存。
- 后端 `model_accounts.rs` 改为保存 `presetKey/providerType/protocol/authStyle/extraEnv`，协议名保留 `anthropic`、`openai-compatible`、`google`。
- Anthropic 调用按 `authStyle` 区分 `x-api-key` 和 Bearer token；Gemini API Key 与 Gemini OAuth Bearer 分开处理。
- 新增 `wridian_google_gemini_oauth_login`：启动 Google OAuth PKCE、监听 localhost callback、交换 token、保存 OAuth JSON 到 Windows Credential Manager，并自动写入 `google-gemini-cli` provider。读 provider 时会在 access token 过期前用 refresh token 刷新。
- 授权登录扩展到 Anthropic/OpenAI：Anthropic 复刻 Hermes Claude PKCE code flow，用户授权后粘贴 code，Wridian 交换 token 并按 Claude Code OAuth 请求头发送 Anthropic Messages；OpenAI 复刻 CodePilot/Codex PKCE loopback flow，固定回调 `http://localhost:1455/auth/callback`，保存 ChatGPT/Codex OAuth token，运行时走 `chatgpt.com/backend-api/codex/responses`。
- 新增 `wridian_delete_model_provider`，取消配置会删除 provider 配置和 Windows Credential Manager 中对应凭据，并让该 provider 回到添加服务列表。

## 验证

- `npm run build` 通过。
- `cmd.exe /c "<vcvars64.bat> && cd /d D:\Coding\Wridian\src-tauri && cargo check"` 通过。
- `cmd.exe /c "<vcvars64.bat> && cd /d D:\Coding\Wridian\src-tauri && cargo test"` 通过，33 个测试全部通过。
- Vite 本地页面用 Playwright/Chrome 验证模型弹窗：桌面 1280x860 下弹窗 900x736，无卡片溢出；窄屏 760x860 下弹窗 736x736，无卡片溢出；Gemini OAuth 连接弹窗 560x605，无溢出。
- UI 修正后再次用 Playwright/Chrome 验证模型弹窗：1280x860 下弹窗 900x736；添加服务标题为“授权登录 / 国内服务 / 第三方API”；待添加 provider 为 12 个；已连接空区高度为 0；无顶部副标题、无弹窗内模型配置块、无卡片溢出。
- 卡片动作修正后用 Playwright/Chrome 验证添加服务区：12 个待添加 provider 均只有右侧“连接”按钮；头像图标数量为 0；底部标签数量为 0；详情框和状态 pill 数量为 0；无卡片溢出。
- Aliyun Bailian Coding Plan 接入后用 Playwright/Chrome 验证添加服务区：待添加 provider 为 13 个；`Aliyun Bailian` 与 `Aliyun Bailian Token Plan` 同时存在，且均只有右侧“连接”按钮；头像图标数量为 0；底部标签数量为 0。
- 供应商 endpoint 修复后执行 `npm run build`、`cargo test --manifest-path src-tauri\Cargo.toml --lib` 通过；新增测试覆盖小米 Token Plan `/anthropic` base URL 会请求 `/anthropic/v1/messages`，OpenAI-compatible base URL 会请求 `/v1/chat/completions`，完整 endpoint 不重复拼接。
- 清空并重建 `dist` 后扫描 `src/`、`src-tauri/`、`dist/`，未命中 `Hermes`、`hermes`、`CodePilot`、`codepilot`。
- Playwright/Chrome 验证 OpenAI-Compatible API 连接小弹窗：按钮包含“测试 / 取消 / 保存服务”，高级入口文案为“查看连接参数”，页面文本未命中 Hermes / CodePilot，弹窗 560x539 未溢出。

## 回退

- 可回退本次涉及的前端 catalog/UI、`model_accounts.rs`、`cocreation.rs` 和新增依赖。
- 旧 `customApi` 迁移包装仍保留；回退不会要求读取或迁移用户明文密钥。
