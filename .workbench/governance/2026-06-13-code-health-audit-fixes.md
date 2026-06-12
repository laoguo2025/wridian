# 2026-06-13 代码健康审查修复

## 背景

本轮针对死代码、硬编码、重复代码、健壮性、安全和技术债审查中确认的 6 类问题做最小修复。

## 变更

- 直接文本读取增加 512KB 门禁，PDF/图片 base64 预览增加 10MB 门禁，避免大文件经 IPC 或上下文注入导致卡顿和内存暴涨。
- DOCX 保存前检查正文 XML，遇到表格、脚注、尾注、批注、修订、绘图、超链接或内容控件等复杂语义时拒绝保存，避免纯文本回写破坏原文档。
- 模型返回的 `fileOperations` 执行时写入 `.wridian/runtime/model-file-operations.jsonl`，记录操作结果和旧目标摘要，作为回退与追溯依据。
- 新增 `src-tauri/src/text_index.rs`，统一中英混合分词和词频统计，替代相关笔记与知识检索中的重复实现。
- 官方 Anthropic 模型别名和 Google Code Assist 请求头增加环境变量覆盖，降低外部协议变化必须改源码的风险。
- 项目地图同步记录新的文件大小门禁、DOCX 保存限制、模型操作审计和模型配置覆盖口径。

## 验证

- `npm run build` 通过。
- `npm audit --audit-level=moderate` 通过，0 vulnerabilities。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，117 个 Rust 测试全部通过。

## 回退

本轮源码改动集中在 `workspace.rs`、`cocreation.rs`、`text_index.rs`、`projects.rs`、`knowledge_ops.rs`、`model_accounts.rs` 和 `lib.rs`；如需回退，可回退本轮提交。已有未纳入本轮的安装包和 `Cargo.toml` 工作树差异未作为本轮修复依据。
