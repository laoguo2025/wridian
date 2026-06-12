# 常见文件查看与 Word 编辑闭环

## 背景

用户要求文件树中的常见格式都能在中间文件区查看，且 `md/txt/docx/doc/word` 这类写作常用文件应尽量可编辑，不能把 Word 文件简单降级为只能本机打开。

## 变更

- 新增 `wridian_preview_file`，让非编辑格式也经过后端路径边界校验后进入中间文件区。
- `md/markdown/txt/docx` 进入正文编辑区并沿现有自动保存链路保存。
- `docx` 使用内置 ZIP/WordprocessingML 纯文本抽取和回写，避免引入外部运行时；当前不承诺保留复杂 Word 样式。
- `pdf` 和图片在中间文件区直接预览；`csv/json/yaml/yml` 以只读文本查看。
- 对话上下文不再对非文本文件直接 `read_to_string`，不可抽取文本的文件会降级为明确文件引用，避免整轮对话失败。

## 参考方案取舍

- MarkItDown 适合作为后续多格式转 Markdown 引擎。
- PaddleOCR 适合后续图片/扫描件 OCR。
- OpenDataLoader PDF 适合后续 PDF 结构化抽取。
- 本轮先完成本地稳定查看/编辑和上下文防失败闭环，避免首版引入重量级外部依赖。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace --lib`
