# 2026-06-09 Cargo MSVC Env

## 背景

普通 PowerShell 中运行 `cargo check --manifest-path src-tauri\Cargo.toml` 会失败，`ring` 和 `vswhom-sys` 的构建脚本找不到 `cl.exe`。同一命令在手工 `vcvars64.bat` 初始化后可以通过，说明根因是 Cargo 子进程缺少 MSVC Build Tools 环境。

## 变更

- 新增 `.cargo/config.toml`，在项目级别注入本机 MSVC Build Tools 路径：
  - `cl.exe`、`lib.exe`、`link.exe`。
  - `INCLUDE`、`LIB`、`LIBPATH`。
  - VS/Windows SDK 版本变量。
- 不修改系统级 PATH，不要求用户打开 VS Developer Shell。

## 验证

- 普通 PowerShell 中直接运行 `cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- 普通 PowerShell 中直接运行 `cargo test --manifest-path src-tauri\Cargo.toml --lib` 通过：5 passed，1 ignored。
- 普通 PowerShell 中直接运行 `npm run tauri -- build --no-bundle` 通过。
- 打包验证中曾遇到 `src-tauri\target\release\wridian.exe` 被运行中的本地测试进程占用；停止该进程后通过，和 MSVC 环境无关。

## 回退

删除 `.cargo/config.toml` 即可回到改动前状态；届时仍需手工先执行 `vcvars64.bat` 或从 VS Developer Shell 运行 Cargo。
