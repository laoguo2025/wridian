# 2026-06-10 Frontmatter Relation Protocol

## 背景

用户要求可以补关系协议，但必须注意文件名不要冲突；需要改名时可以改名，目标是便于系统和用户理解。

用户同时明确：capture / organize 候选箱和知识 ingest MVP 暂不做，后续会先看知识库运营 skill。

## 本轮变更

- 新增长期 owner 文档 `.workbench/doc/wridian-frontmatter-relation-protocol.md`。
- 协议采用带域语义的 `type` 值，避免 `memory`、`knowledge`、`note` 这类宽泛命名。
- 定义作品域类型：`creative_project`、`draft`、`project_element`、`creative_memory`。
- 定义知识域类型：`knowledge_card`、`knowledge_source`、`knowledge_entity`、`knowledge_concept`。
- 定义边界字段：`references_knowledge`、`adopts_knowledge`、`derived_from_knowledge`、`excerpted_from_project`、`abstracted_from_draft`、`distilled_from_memory`。
- 更新项目地图、Workbench 导航和长期文档登记表。

## 验证

- 文档-only 变更；未改产品代码。
- `git diff --check` 通过。

## 回退依据

- 回退新增协议文档，并移除 `.workbench/reademe.md`、项目地图和长期文档登记表中的指针即可。
