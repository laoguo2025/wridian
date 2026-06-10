# 模型账户密钥安全收口

## 背景

用户担心测试用第三方 API token 被硬编码进安装包，导致其他用户下载后可直接使用。

## 排查结论

- 源码、`dist`、`release` 未命中测试 endpoint、模型名或疑似长 API token。
- 本机显示的 endpoint/model 来自 `AppData\Roaming\Wridian\.wridian\model-accounts.json`。
- 配置文件只保存 `baseUrl`、`model`、`keyStored`，真实 API Key 由 Windows 系统凭据保存。

## 本次变更

- 模型账户弹窗增加提示：API Key 只保存在本机系统凭据，不会写入安装包。
- 增加“清除本机凭据”入口，确认后同时删除系统凭据和本机模型账户配置文件。
- 写入 `model-accounts.json` 时不再序列化空 `apiKey` 字段，保留读取旧字段的迁移能力。
- 增加单元测试防止配置文件重新写出 `apiKey` 字段。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml custom_api_settings_file_omits_api_key_field`
- `rg -n "nowcoding|gpt-5\.5|sk-[A-Za-z0-9_-]{16,}" -S src src-tauri dist -g '!src-tauri/target'`

## 回退依据

如需回退，移除 `wridian_clear_custom_api_settings` 命令、前端清除按钮和配置序列化调整即可；旧配置读取逻辑独立保留，不影响回退。
