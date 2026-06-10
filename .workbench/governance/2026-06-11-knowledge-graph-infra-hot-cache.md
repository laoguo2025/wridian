# 2026-06-11 知识图谱基础设施与 hot cache

## 目标

按知识域边界补齐 `claude-obsidian` 借鉴项：一等节点口径、来源链路、反链 / 孤岛 / 重复概念治理提示，以及本地 hot cache 辅助召回。

不做作品元素 / World Info，不替代 `zhishiku-skill`、`chaijie-skill`、`tilian-skill`、`zhengliu-skill` 的知识生产流程。

## 现场接入

- 图谱入口沿用 `wridian_get_knowledge_graph`。
- 对话上下文入口沿用 `wridian_cocreate` 的固定槽位 prompt 编译。
- hot cache 写入 Wridian 运行目录 `.wridian/knowledge-hot-cache.json`，只记录最近显式使用过的知识卡索引信息，不写入知识卡正文，不写入创作记忆树。

## 变更

- 图谱节点从 frontmatter `type/kind/card_type/wridian_type` 归一为 `source/entity/concept/knowledge_card/skill_output/note` 等知识域口径。
- 来源链路字段新增 `source/derived_from/quotes/evidence`，并继续兼容既有 `source_refs/source_url` 等字段。
- 图谱节点返回知识卡反链、作品文件显式引用、重复标题和重复 alias/concept 提示。
- 前端治理视图中“重复待合并”纳入重复 concept/alias；悬浮预览显示被知识卡引用和被作品引用。
- 对话发送时读取 hot cache，按当前稿件、用户输入和显式上下文轻量匹配少量相关知识卡；本轮已经显式 `@` 的知识卡不会重复注入。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_graph`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml hot_cache`

## 回退依据

- 移除 `knowledge_graph.rs` 新增节点元数据、作品反链扫描和类型归一，可恢复旧图谱输出。
- 移除 `cocreation.rs` 的 hot cache 读写和 prompt 槽位，可恢复旧对话上下文编译。
- 删除运行时 `.wridian/knowledge-hot-cache.json` 只会清空辅助召回，不影响知识库文件或作品记忆树。
