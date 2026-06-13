# 2026-06-14 Exact DOCX Episode Chat E2E

## 背景

用户再次截图确认：打开 `第1集.docx`，发送“根据第1集剧情，续写第2集，在作品库里新建个文档保存”后，Wridian 不能只拦截总结回复，也不能保存总结内容；必须真正写出第2集正文并新建文件。

## 根因

- 上一轮只解决了“不要把总结型回复误存文件”，但失败后仍停在提示“模型没有返回 fileOperations”。
- E2E 测试没有按用户截图的完整实例执行：真实 `第1集.docx`、原句、第一轮模型输出总结型回复、后续自动修复并落盘。
- E2E mock 原先只能注入一轮，无法覆盖“第一轮失败、修复轮成功”的真实链路。
- mock 下沉到真实模型链路后暴露出解析兼容缺口：`fileOperations` camelCase 字段未被 Rust 结构直接识别。

## 变更

- E2E mock 改为队列，可为同一次对话的主模型轮、fileOperations 修复轮、正文修复轮分别提供输出。
- mock 入口下沉到 `cocreate_with_model`，不再在共创入口提前返回，确保真实修复链路被 E2E 覆盖。
- E2E 模式下若存在 mock 队列，可使用临时 mock settings，不再需要真实模型账号。
- 当文件树请求缺少可执行 `fileOperations`，且模型只返回总结型回复时，Wridian 会再请求一次“只输出目标新文件完整正文”；拿到独立正文后，本地生成受限 `writeFile` 操作保存。
- 新增后端路径推断：用户原句“根据第1集剧情，续写第2集，在作品库里新建个文档保存”会推断目标 `第2集.md`。
- E2E 夹具改为真实最小 DOCX，而不是把 Markdown 改扩展名。
- 模型解析结构兼容 `fileOperations` camelCase 字段。
- E2E smoke 中新增并通过用户原句实例：打开 `第1集.docx`，第一轮总结型回复，第二轮正文修复，最终创建 `测试/第2集.md`，文件内容为第2集正文，不含“重点推进/文风说明”等对话总结。

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：通过，45 个共创测试全绿。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml e2e --lib`：通过。
- MSVC 环境下 `npm run tauri -- build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\e2e-launch.ps1 -DebugPort 9222 -StopExisting; node scripts\e2e-smoke.mjs`：通过。结果显示 `firstDraftPath` 为 `...\第1集.docx`，覆盖用户截图同句实例。

## 产物

- 安装包：`release/Wridian-0.0.9-x64-setup.exe`

## 回退

- 回退 `src-tauri/src/e2e.rs` 的 mock 队列和真实 DOCX 夹具。
- 回退 `src-tauri/src/cocreation.rs` 的 E2E mock 下沉、正文修复轮、路径推断和 camelCase alias。
- 回退 `scripts/e2e-smoke.mjs` 中针对用户原句实例的 E2E。
