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

## 验证

- 新增单元测试覆盖畸形代码块 JSON 中的 `fileOperations` 恢复。
- 新增单元测试覆盖 `works/第2集.md` 归一化为作品库内 `第2集.md`，且审计记录使用归一化路径。
- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：32 个 cocreation 测试通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`：通过。
- 普通 PowerShell 直接 `npm run tauri -- build` 仍会因未进入 MSVC 环境找不到 `cl.exe` 失败；改为通过 Visual Studio Build Tools `vcvars64.bat` 后 `npm run tauri -- build`：通过。
- 已复制本地测试版：`release\Wridian-0.0.8-test.exe`，SHA256 `7B9F63BD4BF9DFDD3A84BEDE47B353A889AE13C86334A55E123AC3C13D44FEDD`。
- 已复制 NSIS 安装包：`release\Wridian-0.0.8-x64-setup.exe`，SHA256 `BB6076887A6404B0022DB0E2221BC5A66E989FD10BA9618F9F7C09BF908B81E7`。
