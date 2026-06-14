# 2026-06-14 Code Health Review Follow-up Fixes

## 背景

用户要求继续修复项目审查中暴露的死代码、硬编码、重复 fallback、语义冲突、健壮性和安全边界问题。本轮聚焦已定位且能低风险闭环的代码健康问题，不包含既有未提交的 `src-tauri/Cargo.toml` 改动。

## 变更

- 聊天 transcript 持久化补充文件操作块，避免模型实际写文件后导出的会话记录丢失关键行为。
- 聊天文件操作结果、上下文 pill 和引用 pill 改为展示相对路径或文件名，避免把本机绝对路径暴露到 UI、prompt 和归档文本。
- 文件树新增、建文件夹和重命名命令增加 library 边界参数，避免作品库/知识库同名相对路径时语义不清。
- 移除前端新文档本地写入 fallback，统一由后端文件工具链处理，避免双链路重复写入和行为漂移。
- Tauri 打包资源收窄到 knowledge-health references，避免把 Python 脚本作为运行时资源继续携带。
- 打开目录权限去掉默认 works 根兜底，只保留当前显式允许的目录集合。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，149 个 Rust 测试通过。
- `npm audit --omit=dev` 为 0 个漏洞。
- `cargo audit` 当前环境未安装，无法执行 Rust 依赖漏洞扫描。

## 回退

本轮改动集中在聊天文件操作、文件树命令边界、打包资源和目录打开权限。若需要回退，可 revert 本轮提交；回退后需重新验证聊天新建文件、知识库/作品库文件树新增重命名、会话导出和 Tauri build。
