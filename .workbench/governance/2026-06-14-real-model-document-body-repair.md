# 2026-06-14 Real Model Document Body Repair

## 背景

用户安装新版后仍复现：打开 `第1集.docx`，发送“根据第1集剧情，续写第2集，在作品库新建文档保存”，Wridian 仍提示模型没有返回可执行文件操作。

## 现场结论

- 安装版不是旧包。安装路径 `C:\Users\Administrator\AppData\Local\Wridian\Wridian.exe` 的时间与本轮打包时间一致。
- 用户真实模型没有返回 `fileOperations`，且之前的正文修复轮仍复用普通共创 JSON 协议，容易继续被工具协议牵制，最终落到拦截提示。
- E2E 虽覆盖了多轮 mock，但没有证明真实 OpenAI-compatible 弱模型路径不再受 `response_format/json_object` 约束。

## 变更

- 对 OpenAI-compatible 模型新增专门的纯正文生成请求：不带 `response_format`，system prompt 只要求输出可保存的新文档正文，不要求 JSON 或 fileOperations。
- 新建文档请求第一轮和 fileOperations 修复轮失败后，Wridian 会进入正文生成轮；拿到正文后由本地受限 `writeFile` 写入目标文件。
- 保留总结型回复门槛：只有正文以标题、集标题或 fenced 内容开头才允许写入，避免再次把“重点推进/文风说明”保存为正文。
- 保持 E2E 的真实 `第1集.docx` 场景和多轮 mock 队列，继续验证同一句请求最终创建 `第2集.md`。

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：通过，45 个共创测试全绿。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml e2e --lib`：通过。
- MSVC 环境下 `npm run tauri -- build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\e2e-launch.ps1 -DebugPort 9222 -StopExisting; node scripts\e2e-smoke.mjs`：通过，结果显示 `firstDraftPath` 为 `...\第1集.docx`。

## 产物

- 安装包：`release/Wridian-0.0.9-x64-setup.exe`

## 回退

- 回退 `src-tauri/src/cocreation.rs` 中 `generate_document_body_with_openai_compatible`、纯正文 prompt 和正文修复轮调用方式。
