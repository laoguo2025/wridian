# 2026-06-08 File Autosave And Custom API

## 目标

- 正文编辑区接入真实文件内容。
- 正文变更后自动保存，并保留快捷键保存。
- 模型账户先实现 OpenAI-compatible 自定义 API 配置和连接测试。

## 回退依据

- 本轮改动集中在前端 UI、Tauri 命令、Rust 依赖和最小 workbench 文档。
- 如需回退，撤销本轮提交即可恢复到初始桌面壳。

## 验证

- 已通过：`npm run build`
- 已通过：`cargo fmt`
- 未通过：`cargo check`，失败原因为本机缺少 MSVC `link.exe`。

