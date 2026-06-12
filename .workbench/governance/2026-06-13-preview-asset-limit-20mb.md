# 2026-06-13 预览文件上限调整为 20MB

## 目标

- 将 PDF 和图片预览的 base64 IPC 大小门禁从 10MB 调整为 20MB。
- 保持文本读取和对话上下文注入的 512KB 门禁不变。

## 变更

- `MAX_PREVIEW_ASSET_BYTES` 调整为 `20 * 1024 * 1024`。
- 后端测试补充 20MB 边界：等于上限允许预览，超过上限拒绝。
- 项目地图同步更新长期事实。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace_text_and_asset_preview_reject_oversized_files` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，117 个测试全部通过。
- `npm run build` 通过。

## 回退

- 将 `MAX_PREVIEW_ASSET_BYTES` 恢复为 `10 * 1024 * 1024`，并同步恢复项目地图即可。
