# Library Picker And Uninstall Data Cleanup

## 问题

- 文件区工具栏的文件夹图标在已配置作品库时会直接打开当前路径；用户预期是始终弹出目录选择器。
- 卸载旧版时勾选“删除应用程序数据”后，模型设置仍显示旧 Base URL 和模型名。

## 排查

- 前端按钮调用 `openCurrentLibraryFolder`，该函数在已有根目录时调用 opener 打开路径，未进入目录选择器。
- 本机存在 `%APPDATA%\Wridian\.wridian\model-accounts.json`，其中保存 Base URL 和模型名；源码和打包产物未发现 `nowcoding`、`gpt-5.5` 或 API Key 字面量。
- Tauri NSIS 默认清数据路径是 `$APPDATA\${BUNDLEID}` 和 `$LOCALAPPDATA\${BUNDLEID}`，当前 bundle id 为 `ai.wridian.app`；Wridian 后端实际数据目录为 `$APPDATA\Wridian`，两者不一致。
- API Key 通过 Windows Credential Manager 保存，目标名由 keyring 默认规则生成为 `custom-api-key.ai.wridian.app`。

## 变更

- 工具栏文件夹图标始终调用目录选择器，并把 tooltip/aria 文案统一为“选择作品库文件夹 / 选择知识库文件夹”。
- 增加 NSIS post-uninstall hook：仅当卸载器的删除应用数据复选框被勾选且不是更新模式时，额外删除 `$APPDATA\Wridian`、`$LOCALAPPDATA\Wridian` 和 `custom-api-key.ai.wridian.app` 凭据。
- 后续新写入的模型配置文件在 API Key 不写入 JSON 时不再序列化 `apiKey: null`。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，30 个 Rust 单测全过。
- `npm run tauri -- build` 通过，已重新生成 NSIS 安装包。
- 已将 `src-tauri\target\release\wridian.exe` 复制为 `release\Wridian-0.0.2-test.exe`，将新 NSIS 安装包复制为 `release\Wridian-0.0.2-x64-setup.exe`。
- 生成的 NSIS 脚本包含 `src-tauri\nsis-hooks.nsh`，且 hook 位于读取 `DeleteAppDataCheckboxState` 之后。
- 源码、dist 和 release 范围扫描未发现 `nowcoding`、`gpt-5.5` 或形如 `sk-...` 的密钥字面量。
