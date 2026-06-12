# Wridian Metadata Index

## 定位

Metadata Index 是 Wridian 作品库和知识库的 Markdown 元数据解析底座。

它统一解析文件、frontmatter、aliases、tags、wikilinks、embeds、backlinks 和 unresolved links，避免知识图谱、相关笔记、知识库体检和跨域桥接各自重复扫描。

## 当前 owner

- 后端实现：`src-tauri/src/metadata_index.rs`
- Tauri 命令：`wridian_get_metadata_index`
- 知识图谱消费：`src-tauri/src/knowledge_graph.rs`
- 知识缓存/检索/fold/体检/低风险修复：`src-tauri/src/knowledge_ops.rs`
- 前端类型：`src/appTypes.ts`

## 边界

- 作品库和知识库都进入索引，但语义不混用。
- 作品库索引用于项目创作、相关稿件和作品到知识的沉淀候选。
- 知识库索引用于知识图谱、知识库体检、知识卡召回和 skill 化流水线。
- 两域连接只通过显式 frontmatter 关系字段和 wikilink 解析结果表达。
- Metadata Index 仍是按需解析的事实源。
- 知识域有持久化热缓存层：`wridian_refresh_knowledge_cache` 会写入知识库 `.wridian/knowledge-manifest.json`。
- manifest 用 `mtime + len` 复用未变文件的 `sha256` 和 token 统计；链接、反链和断链计数仍每次来自最新 Metadata Index。
- BM25 检索命令 `wridian_search_knowledge_bm25` 只针对知识库，返回知识卡路径、分数、摘要、标签和链接信号。
- `wridian_update_knowledge_hot_cache` 生成知识库 `hot.md`；`wridian_fold_knowledge_cache` 在 `00知识库治理/folds/` 生成抽取式 fold。
- `wridian_run_knowledge_health_check` 是用户层一键体检入口，内部刷新 manifest、更新 `hot.md`、生成 fold、运行健康扫描并写入本次时间戳命名的 `00知识库治理/知识库体检-YYYYMMDDTHHMMSS*.md`。
- `wridian_fix_knowledge_health_low_risk` 是用户层一键修复入口，只执行补缺目录、补治理模板等低风险确定性修复；语义改写、合并、归档和冲突处理进入报告待确认清单。
- `wridian_audit_knowledge_health` 保留为底层审计函数/命令，复用 Metadata Index 与知识 manifest，返回知识库健康分、skill 化成熟度、结构统计、主要问题和 skill 化候选。

## 解析规则

- Markdown 文件范围：`md`、`markdown`。
- 跳过隐藏目录、`.git`、`node_modules`、`.wridian`、`.wridian-trash`、符号链接和过大文件。
- wikilink 支持：`[[target]]`、`[[target|display]]`、`[[target#section]]`、`![[embed]]`。
- frontmatter 关系字段中的 wikilink 会带上 `frontmatter_field`。
- aliases/tags 从 frontmatter 读取，参与链接解析和后续筛选。
- 链接解析优先当前文件相对目录，再按标题、相对路径和 alias 解析；同优先级多命中时标记 ambiguous。

## 后续演进

- P1：将 Relevant Notes 切到 Metadata Index。
- P2：扩展 heading/block link、Markdown 内部链接和属性 schema 校验。
- P3：语义检索层接入本地 embedding/provider 时，必须复用 manifest 和 Metadata Index，不得绕开知识域边界。
