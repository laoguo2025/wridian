# 2026-06-12 Inline PDF And Image Preview

## 背景

用户反馈 PDF、PNG、JPG 在中间文件编辑区不能直接预览，现状表现为图片破图或 PDF 占位。

## 排查

- 当前前端使用 Tauri `convertFileSrc(path)` 生成本地资源 URL，再交给 `img` / `iframe`。
- 项目文件可来自用户选择的作品库和知识库目录，直接依赖 asset 协议容易受本地路径授权和 CSP 限制影响。
- 本机 Obsidian 安装包中可见 `lib/pdfjs`，说明其 PDF 预览是应用内能力；Wridian 当前先采用受控工作区读取加内联预览 URL 的闭环，不放宽全盘本地文件协议。

## 变更

- 后端新增 `wridian_preview_asset`，只允许读取当前作品库/知识库范围内的 PDF 和图片资源。
- 后端按扩展名返回 MIME 类型和 base64 data URL。
- 前端打开 PDF/图片时调用预览资源命令，并在中间区域直接渲染。
- PDF iframe CSP 增加 `data:`，图片继续走 `img-src data:`。
- 中间预览区补充满高布局和加载失败提示。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `git diff --check`

## 回退

回退本次提交即可恢复到 `convertFileSrc` 预览路径；不会影响文本文件编辑、保存、文件树扫描或系统程序打开入口。
