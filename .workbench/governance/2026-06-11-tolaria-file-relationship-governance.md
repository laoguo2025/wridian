# 2026-06-11 Tolaria 文件关系治理

## 目标

按 Tolaria 的文件系统唯一真相思路，补 Wridian 本地 Markdown 关系治理层；明确排除作品元素和 World Info。

## 接入点

- 文件树重命名命令：`src-tauri/src/workspace.rs`
- 知识图谱扫描命令：`src-tauri/src/knowledge_graph.rs`
- 图谱抽屉：`src/knowledge/KnowledgeGraphDrawer.tsx`

## 变更

- 文件或文件夹重命名后，扫描同一作品库或知识库内 Markdown，并修复受影响 wikilink。
- 支持短链接、别名链接、路径式链接和 frontmatter 内 wikilink。
- 发生实际修复时，在库根 `.wridian/link-repair/` 写 JSON 回滚记录。
- 知识图谱返回统一 frontmatter 关系结构：字段名、源文件、目标文件、关系类型、是否双向。
- 固定关系字段保留语义；非固定字段作为普通关系进入图谱。
- 支持 `type: Type` Markdown 类型定义的图标、颜色、排序和默认字段。
- 增加只读治理视图：无来源、未被引用、采纳未沉淀、孤岛、重复标题、陈旧高频。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml`
  - 38 passed
- `npm run build`
  - passed

## 回退依据

- 本轮代码改动均在 Wridian 工作区内。
- 用户数据侧的实际重命名链接修复会写 `.wridian/link-repair/*.json`，记录每个被改 Markdown 的 before/after，可用于人工回滚。
- 若功能需代码级回退，回退本次提交即可；不涉及外部发布或 push。
