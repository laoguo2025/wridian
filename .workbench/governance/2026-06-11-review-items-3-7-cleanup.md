# 2026-06-11 审查第 3-7 项治理

## 范围

- 第 3 项：项目地图里旧聊天输入组件路径仍指向 `CopilotPromptEditor`。
- 第 4 项：OpenAI-compatible 请求体按供应商名写死兼容逻辑，缺少显式配置边界。
- 第 5 项：OpenAI OAuth 回调端口固定为 1455，和 Gemini 的临时端口回退策略不一致。
- 第 6 项：记忆树 legacy 迁移在热路径重复执行，且旧文件读取失败会被当作空内容。
- 第 7 项：若干前端 catch 静默吞错，用户无法知道降级或失败原因。

## 变更

- 项目地图更新为当前 `WridianPromptEditor` 入口，并同步 OpenAI OAuth 与 OpenAI-compatible 请求配置口径。
- OpenAI-compatible 请求体改为读取 provider `extraEnv`：`WRIDIAN_OPENAI_COMPAT_MAX_TOKENS_FIELD`、`WRIDIAN_OPENAI_COMPAT_OMIT_TEMPERATURE`、`WRIDIAN_OPENAI_COMPAT_THINKING`。默认不再按供应商名、Base URL 或模型名猜测特殊参数。
- 模型设置页为通用 OpenAI-compatible provider 开放 `env_overrides` 编辑，保存和测试共用同一组参数。
- OpenAI OAuth 先监听 `127.0.0.1:1455`，端口占用时回退临时本机端口，并用实际监听端口构造授权和换 token 的 redirect URI。
- 记忆树 legacy 迁移写入 `.legacy-migration-complete` marker；marker 存在后不再重复扫描旧结构，旧文件读取失败时返回明确错误。
- 前端模型账户加载、工作区初始化、文件加入对话降级、复制失败、聊天续接失败、Lexical 输入器错误均改为可见错误反馈。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml openai_compatible --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml legacy_memory_migration_runs_once_after_marker_is_written --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`

## 回退

- 回退本提交即可恢复旧路径口径、固定 OpenAI OAuth 端口、默认 OpenAI-compatible 请求体和原先错误处理。
- 用户已有 provider 配置不会迁移或删除；新增的 OpenAI-compatible env 字段只影响之后保存或显式编辑的 provider。
