# 2026-06-12 Obsidian Style File Viewer

## 背景

用户要求不要只处理 PDF、PNG、JPG，要挖本机 Obsidian 对各种文件的预览、编辑、翻页、搜索、缩放方式，并借鉴复刻到 Wridian 中间区域。

## 排查

- 本机 Obsidian 安装目录为 `%LOCALAPPDATA%\Programs\Obsidian`。
- 解包 `resources/obsidian.asar` 到 `.workbench/runtime/obsidian-asar` 临时目录后确认：
  - 包内包含 `lib/pdfjs`，PDF 查看不是单纯外部打开。
  - 样式中存在 `pdf-container`、`pdf-viewer-container`、`pdf-findbar`、`pdf-toolbar` 等结构。
  - 图片查看使用中间区域容器与最大宽度约束。
  - 文档搜索使用顶部内嵌查找条、命中计数和高亮。
- PDF.js 官方示例确认基础渲染链路为 `getDocument`、`getPage`、`getViewport({ scale })`、`page.render(...)`。

## 变更

- 新增 `src/viewer/FilePreviewViewer.tsx`，把文件查看逻辑从 `App.tsx` 拆出。
- PDF 使用 `pdfjs-dist` 按需加载，支持翻页、页码输入、缩放、全文搜索并跳转到命中页。
- 图片支持适应窗口、100%、放大和缩小。
- CSV/JSON/YAML 等只读文本支持搜索、上下跳转和高亮。
- Office 二进制类文件仍保持安全提示和本机程序打开入口，避免危险写入。
- CSP 增加 `worker-src` 和 `script-src blob:` 以支持 PDF.js worker 和按需 chunk。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `git diff --check`

## 回退

回退本次提交即可恢复到上一版 iframe/img/text 预览。后端 `wridian_preview_asset` 不受本次前端查看器拆分影响。
