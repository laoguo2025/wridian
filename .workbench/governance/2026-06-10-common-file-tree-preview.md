# Common File Tree Preview

## 背景

用户要求文件树像 Obsidian 一样显示常见文件格式，并带格式标注；`md`、`txt` 可看可编辑，其他常见格式只能看，不能进入文件编辑区编辑。

## 变更

- 后端文件树扫描从“只显示可写作文本文件”拆成“可显示文件”和“可编辑文件”。
- 可显示文件包含 `md/markdown/txt`、Office/WPS 文档、PDF、常见图片、表格、PPT 和 JSON/YAML。
- 可编辑文件只允许 `md/markdown/txt`，打开和保存命令继续拒绝 PDF、图片、DOCX、WPS、PPT 等非文本编辑格式。
- 前端点击非可编辑文件时进入只读预览状态：图片直接预览，PDF 尝试内嵌预览，Office/WPS/PPT 等提供“用本机程序打开”。
- 文件树已有扩展名标注继续复用，预览文件也会保持选中高亮。

## 非变化

- 新建文件仍默认创建 `md`，不创建 Office/WPS 格式。
- 文档原格式编辑不在本轮实现；DOCX/WPS 可编辑需要单独文档编辑链路。

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace_tree_displays_common_files_but_edits_only_text_notes`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml`：31 个测试通过。
