# 2026-06-12 Metadata Index P0

## 目标

建立 Work/Knowledge Metadata Index，统一解析文件、frontmatter、wikilinks、aliases、backlinks 和 unresolved links。

## 变更

- 新增 `src-tauri/src/metadata_index.rs`，提供共享索引和 `wridian_get_metadata_index` 命令。
- 知识图谱改为消费 Metadata Index，保留原前端响应结构。
- 新增前端 Metadata Index 类型定义。
- 新增长期 owner 文档 `.workbench/doc/wridian-metadata-index.md`，并更新 workbench 导航与登记表。

## 验收

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`
- 结果：79 passed。

## 回退

- 回退 `src-tauri/src/metadata_index.rs`、`src-tauri/src/knowledge_graph.rs`、`src-tauri/src/lib.rs`、`src/appTypes.ts` 和本次 workbench 文档改动即可恢复到旧知识图谱扫描实现。
