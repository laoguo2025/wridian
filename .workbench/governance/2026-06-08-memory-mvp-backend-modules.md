# 2026-06-08 Memory MVP Backend Modules

## 目标

- 避免 `src-tauri/src/lib.rs` 成为大文件。
- 拆分后端职责，再继续写作记忆 MVP。

## 变更

- `lib.rs` 缩减为 Tauri 启动和命令注册。
- 新增 `runtime.rs`、`workspace.rs`、`model_accounts.rs`、`memory.rs`。
- 前端记忆抽屉接入真实记忆状态、待确认记忆、记住和忽略动作。

## 验证

- 已通过：`cargo fmt`
- 已通过：`npm run build`
- 已验证：浏览器预览里共创输入可生成待确认记忆，抽屉显示“记住/忽略”动作。
- 未通过：`cargo check`，失败原因为本机缺少 MSVC `link.exe`，尚未进入业务代码编译阶段。
