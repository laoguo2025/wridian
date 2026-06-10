# 安装版文件夹选择修复

## 背景

0.0.1 安装版左侧提示“请选择本地文件夹”，但用户无法选择文件夹。

## 根因

前端已调用 `@tauri-apps/plugin-dialog` 的 `open({ directory: true })`，但 Tauri capability 未授权 dialog open 命令。安装版运行时会拦截目录选择 IPC。

## 本次变更

- `src-tauri/capabilities/default.json` 增加 `dialog:allow-open`。
- 左侧文件树为空时显示可点击的“选择作品库文件夹 / 选择知识库文件夹”按钮，不再只依赖右上角小图标入口。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- 在 MSVC 环境中执行 `npm run tauri build`
- 更新 `release/Wridian-0.0.1-test.exe`
- 更新 `release/Wridian-0.0.1-x64-setup.exe`
- 扫描 `release` 和 `dist`，未命中测试 API endpoint、模型名或疑似长 token。

## 回退依据

如需回退，移除 capability 中的 `dialog:allow-open` 和空状态选择按钮即可；不影响已有文件读写命令。
