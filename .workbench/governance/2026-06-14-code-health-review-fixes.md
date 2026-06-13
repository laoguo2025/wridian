# 2026-06-14 Code Health Review Fixes

## Context

修复本轮代码健康审查确认的 5 类问题：卸载清数据整库删除风险、已有节点链接/重解析点操作风险、正文生成失败自动底稿假成功、DOCX XML 生成重复实现，以及 Google 公共桌面 OAuth 默认值的 secret-like 命名。

## Changes

- NSIS hook 生成脚本改为对默认知识库和用户配置知识库只清 Wridian 运行产物：`.wridian`、`.wridian-trash`、`hot.md`、`00知识库治理/folds` 和体检报告；不再 `RMDir /r` 删除整个 `Wridian知识库` 根目录。
- 生成脚本加入禁止整库删除字符串的自检，避免后续重新生成 hook 时回潮。
- 既有文件树节点解析入口拒绝最终目标为 symlink 或 Windows reparse point，覆盖 UI 绝对路径入口和模型相对路径入口。
- 新文档正文生成失败时不再自动写入本地草稿底稿；改为停止写入并提示模型未返回可保存正文。
- 新增共享 `docx_xml` helper，生产 DOCX 写入、E2E 夹具和测试共用同一最小 WordprocessingML 生成逻辑。
- Google OAuth 默认公共桌面客户端凭据常量改为 public desktop client credential 命名，并保留环境变量覆盖口径。

## Validation Plan

- `node .workbench/tools/generate-nsis-hooks.mjs`
- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts/cargo-msvc.ps1 check --manifest-path src-tauri/Cargo.toml`
- `powershell -ExecutionPolicy Bypass -File scripts/cargo-msvc.ps1 test --manifest-path src-tauri/Cargo.toml --lib`
- `npm audit --omit=dev`

## Rollback

回退本轮提交即可恢复旧卸载 hook、旧正文 fallback、旧节点解析和旧 DOCX helper 分布。本轮不迁移用户数据，不改外部服务配置，不发布或推送。
