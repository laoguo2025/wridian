# 2026-06-14 E2E Exe Control Entry

## 背景

用户询问如何让 Codex 直接控制测试版 exe 做真实功能测试并找 bug。结论是不用给正式版开公开 API，而是在测试模式下组合 WebView2 远程调试和受环境变量保护的 Tauri E2E 控制命令。

## 变更

- `WRIDIAN_DATA_DIR` 可覆盖 Wridian 数据目录，用于隔离测试数据。
- 新增 `src-tauri/src/e2e.rs`：
  - `wridian_e2e_status`：返回测试模式和数据目录状态。
  - `wridian_e2e_prepare_fixture`：仅 `WRIDIAN_E2E=1` 时可用，创建临时作品库/知识库夹具并写入工作区配置。
  - `wridian_e2e_set_next_cocreation`：仅 `WRIDIAN_E2E=1` 时可用，注入下一轮共创 mock 输出。
- 共创链路在 `WRIDIAN_E2E=1` 且存在 mock 输出时优先消费一次 mock，不读取真实模型账号。
- 前端仅在后端确认 E2E 开启时挂载 `window.__WRIDIAN_E2E__`，用于 Playwright 控制真实 Tauri WebView。
- 新增 `scripts/e2e-launch.ps1`：以隔离数据目录、`WRIDIAN_E2E=1` 和 WebView2 remote debugging port 启动真实 exe。
- 新增 `scripts/e2e-smoke.mjs`：连接 WebView2 CDP，准备夹具，验证对话驱动的作品库文件树增/改/删、对话驱动的知识库文件树增/改/删、Markdown 表格渲染、划词添加到对话框后发送，以及正文 inline diff 基本链路。

## 安全边界

- 普通启动默认关闭 E2E 控制命令。
- E2E 命令不开放网络监听端口，仍走 Tauri IPC。
- WebView2 调试口只由启动脚本显式开启，建议只用于本机测试。
- 测试数据目录默认在 `.workbench/runtime/e2e-data`，不污染真实用户数据。

## 使用

1. 先构建真实 exe：`npm run tauri -- build`，Windows 普通 PowerShell 需要先进入 MSVC 环境。
2. 启动测试 exe：`powershell -ExecutionPolicy Bypass -File scripts\e2e-launch.ps1 -DebugPort 9222`
3. 如未安装 Playwright，先执行：`npm install --save-dev playwright`
4. 执行 smoke：`node scripts\e2e-smoke.mjs`

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml e2e --lib`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：通过，42 个共创测试全绿。
- `npm run tauri -- build` in MSVC environment：通过，生成 MSI 和 NSIS 包。
- `node scripts\e2e-smoke.mjs` against real `src-tauri\target\release\wridian.exe` launched by `scripts\e2e-launch.ps1`：通过，截图写入 `.workbench/runtime/e2e-artifacts/wridian-e2e-smoke.png`。

## 回退

- 删除 `src-tauri/src/e2e.rs`、`lib.rs` 中 E2E 命令注册、`runtime.rs` 的 `WRIDIAN_DATA_DIR` 覆盖、`App.tsx` 中 `window.__WRIDIAN_E2E__` 挂载，以及两个 `scripts/e2e-*` 脚本即可恢复。
