# 2026-06-12 File Viewer Obsidian Parity Fix

## 背景

用户用截图对比指出上一轮文件查看器没有仔细复刻 Obsidian：Wridian 的 PDF 加载失败，图片和 PDF 都被放进带大标题、格式角标、外部打开按钮和卡片边框的小面板里。

## 对比结论

- Obsidian 的 PDF/图片查看是内容优先：标题很克制，工具栏低存在感，内容直接占据中间工作区。
- Wridian 上一版仍沿用写作稿纸宽度和文件标题区，导致预览区域过窄、顶部视觉噪声大。
- PDF.js 不能在 Tauri 打包环境里稳定通过 data URL `url` 参数加载后端资源，应传 `Uint8Array`。

## 变更

- 预览态新增 `paper-preview`，脱离写作稿纸 760px 宽度，直接占满中间区域。
- 移除预览顶部大标题、格式角标和“用本机程序打开”大按钮。
- PDF 和图片各自显示低调居中的文件标题和工具条。
- PDF 加载路径从 `getDocument({ url: dataUrl })` 改为 data URL 解码后的 `getDocument({ data: Uint8Array })`。
- 预览容器去掉卡片边框和厚重阴影，让 PDF 页面和图片本身成为视觉主体。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `git diff --check`

## 回退

回退本次提交即可恢复上一版查看器布局和 PDF data URL 加载方式。
