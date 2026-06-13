# 文件树操作恢复修复

## 背景

用户要求对作品库/知识库文件树执行新增、改名、删除等操作时，模型可能返回带代码块的 JSON 或存在轻微格式问题的 JSON。此前解析失败后只恢复 `reply` 和 `edits`，会丢弃 `fileOperations`，导致文件树操作没有落地，且工具载荷可能作为聊天正文展示。

## 参考

- `obsidian-copilot` 的 Composer 将写文件结果放入明确的 `writeFile` 工具块，并在聊天渲染中分离工具块和普通文本。
- `claudian` 将 `Write/Edit` 归类为文件写入工具，并通过写入/编辑渲染器展示结果和 diff，而不是把工具参数当普通回复。

## 本轮变更

- 畸形 JSON 兜底恢复新增 `fileOperations` 解析，能恢复的文件树操作继续进入后端执行器。
- 当恢复到文件树操作且没有可靠 `reply` 时，使用短结果文案，避免把 JSON 载荷显示到对话区。
- 文件树操作执行前剥离重复库名前缀，例如 `works/第2集.md` 在作品库中按 `第2集.md` 执行；仍由现有工作区边界校验拦截绝对路径和越界路径。
- 追加处理未闭合/坏围栏的结构化输出：只要文本中已经出现 `reply`、`edits`、`fileOperations` 或 `memories` 字段，就进入结构化恢复路径；若无法恢复完整操作，也隐藏原始工具载荷，避免 JSON 直接进入聊天气泡。

## 验证

- 新增单元测试覆盖畸形代码块 JSON 中的 `fileOperations` 恢复。
- 新增单元测试覆盖 `works/第2集.md` 归一化为作品库内 `第2集.md`，且审计记录使用归一化路径。
- 新增单元测试覆盖截图中的未闭合坏围栏/截断结构化输出：可恢复完整 `fileOperations` 时执行，无法恢复时不展示原始 JSON。
- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：34 个 cocreation 测试通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`：通过。
- 普通 PowerShell 直接 `npm run tauri -- build` 仍会因未进入 MSVC 环境找不到 `cl.exe` 失败；改为通过 Visual Studio Build Tools `vcvars64.bat` 后 `npm run tauri -- build`：通过。
- 已复制本地测试版：`release\Wridian-0.0.8-test.exe`，SHA256 `2EB87D554CF24280D47DE65CA0881C9D6BA6675724610C4ACF6BC12E536EBE0D`。
- 已复制 NSIS 安装包：`release\Wridian-0.0.8-x64-setup.exe`，SHA256 `BDA05CA05EE6F16474DB9E91FB1E717542E14625F30A50EFDA5DD2AEEA548F46`。
