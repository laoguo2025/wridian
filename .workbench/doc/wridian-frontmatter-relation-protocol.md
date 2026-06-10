# Wridian Frontmatter Relation Protocol

## 定位

本文是 Wridian 作品域、知识域和两域边界的 frontmatter 关系协议唯一 owner 文档。

协议只定义 Markdown 文件如何声明身份、归属和关系；不定义 capture / organize 候选箱，也不定义知识 ingest 流程。

## 命名原则

- 字段名使用小写 snake_case，便于系统稳定解析。
- `type` 值必须带域语义，避免使用 `memory`、`knowledge`、`note` 这类宽泛名字。
- 用户可见文件名优先中文清晰，系统稳定识别依赖 frontmatter，不依赖文件名。
- 需要改名时，优先把文件名改成用户能理解的中文名；跨文件关系使用 `id` 或相对路径，不使用易变标题。
- 不同域之间只通过关系字段连接，不复制同一份事实。

## 通用字段

```yaml
id: "stable-human-readable-id"
type: "draft | creative_project | project_element | creative_memory | knowledge_card | knowledge_source | knowledge_entity | knowledge_concept | skill_output"
status: "draft | active | archived | deprecated"
title: "用户可见标题"
aliases: []
created_at: "YYYY-MM-DD"
updated_at: "YYYY-MM-DD"
```

- `id` 是系统稳定引用；建议由相对路径 slug 或用户确认的短 ID 生成。
- `title` 可以改，不能作为唯一关系键。
- `status: deprecated` 表示仍可读但不再作为默认上下文。

## 作品域类型

### creative_project

作品项目根说明或项目压缩记忆可使用。

```yaml
type: creative_project
project_id: "project:<slug>"
status: active
```

### draft

章节、场景、分集、剧本段落、人物小传草稿等稿件可使用。

```yaml
type: draft
project_id: "project:<slug>"
draft_kind: "prose | screenplay | outline | character_bio | setting"
belongs_to: "project:<slug>"
uses_elements: []
references_knowledge: []
```

### project_element

人物、地点、物件、伏笔、禁区、风格、世界观等作品相关元素可使用。

```yaml
type: project_element
project_id: "project:<slug>"
element_kind: "character | location | prop | rule | style | taboo | plot_thread | foreshadowing | worldbuilding"
belongs_to: "project:<slug>"
appears_in: []
references_knowledge: []
derived_from_knowledge: []
```

### creative_memory

创作记忆树里的项目记忆、规则、压缩记忆可使用。

```yaml
type: creative_memory
project_id: "project:<slug>"
memory_kind: "project_summary | compressed | rule | progress | boundary | decision"
belongs_to: "project:<slug>"
supports: []
references_knowledge: []
```

## 知识域类型

### knowledge_card

用户可复用的通用知识卡。

```yaml
type: knowledge_card
knowledge_kind: "method | fact | reference | trope | research | style | checklist"
status: active
source_refs: []
source: []
derived_from: []
quotes: []
evidence: []
related_to: []
used_by_projects: []
review_status:
conflicts_with: []
uncertainty:
```

可追溯来源字段口径：

- `source`：素材来源字段，可指向原始资料、网页、书、PDF、拆解来源。
- `derived_from`：从某来源、拆解报告或上游知识产物提炼而来。
- `quotes`：直接引用或摘录来源。
- `evidence`：支撑该知识卡判断的依据材料。
- 旧字段 `source_refs/source_ref/source_url/source_title` 继续有效。

`zhishiku-skill` 体检产物字段：

- `review_status`：知识库运维给出的只读体检状态，如 `待核查`、`有冲突`、`需合并`、`已确认`。Wridian 只展示，不自行判定。
- `conflicts_with`：与本卡存在观点冲突或适用条件冲突的知识卡列表。
- `uncertainty`：不确定性说明、适用条件缺口或需要补证的原因。
- 中文旧字段 `体检状态/治理状态/核查状态`、`冲突对象/冲突卡片`、`不确定性/待核查` 继续可读，用于兼容 `zhishiku-skill` 的中文卡片。

### knowledge_source

来源资料、摘录、书籍、网页、访谈、文档等原始或半原始材料。

```yaml
type: knowledge_source
source_kind: "book | article | web | interview | note | document"
status: active
source_url:
source_title:
extracts_to: []
```

### knowledge_entity

知识图谱里的实体，如人物原型、地点、组织、术语、物件等。

```yaml
type: knowledge_entity
entity_kind: "person | place | organization | object | term | event"
status: active
source_refs: []
related_to: []
```

### knowledge_concept

知识图谱里的概念、主题、方法论、风格规则等。

```yaml
type: knowledge_concept
concept_kind: "theme | method | genre_rule | style | theory | constraint"
status: active
source_refs: []
related_to: []
```

### skill_output

`zhishiku-skill`、`chaijie-skill`、`tilian-skill`、`zhengliu-skill` 产生的拆解、提炼、蒸馏或中间报告。

```yaml
type: skill_output
skill_kind: "chaijie | tilian | zhengliu | zhishiku | report"
status: active
source: []
derived_from: []
evidence: []
extracts_to: []
```

## 边界关系字段

### 从知识到作品

```yaml
references_knowledge: ["knowledge:<id-or-path>"]
adopts_knowledge: ["knowledge:<id-or-path>"]
derived_from_knowledge: ["knowledge:<id-or-path>"]
```

- `references_knowledge`：本文件引用知识卡，但未把它变成作品设定。
- `adopts_knowledge`：用户确认把知识卡采纳进作品域。
- `derived_from_knowledge`：已改写成作品设定、人物边界或规则。

### 从作品到知识

```yaml
excerpted_from_project: ["project:<slug>"]
abstracted_from_draft: ["draft:<id-or-path>"]
distilled_from_memory: ["creative_memory:<id-or-path>"]
```

- `excerpted_from_project`：从作品项目摘录到知识域。
- `abstracted_from_draft`：从稿件抽象成通用知识。
- `distilled_from_memory`：从创作记忆沉淀成知识卡。

## 文件命名建议

### 作品域

- 项目压缩记忆：`compressed.md`
- 项目长期记忆：`project.md`
- 人物元素：`人物 - <姓名>.md`
- 地点元素：`地点 - <名称>.md`
- 规则元素：`规则 - <主题>.md`
- 禁区元素：`禁区 - <主题>.md`

### 知识域

- 知识卡：`<分类>/<标题>.md`
- 来源资料：`sources/<来源标题>.md`
- 实体：`entities/<实体名>.md`
- 概念：`concepts/<概念名>.md`
- hot cache：运行时缓存 `.wridian/knowledge-hot-cache.json`
- 索引：`index.md`
- 体检报告：`00知识库治理/知识库体检-YYYY-MM-DD.md`

## Type 文档

知识库可以使用 `type: Type` 的 Markdown 文件作为通用知识类型定义。Type 文档只服务知识卡、来源、概念、实体、方法论等通用知识类型，不用于人物卡或 World Info。

```yaml
type: Type
title: "knowledge_card"
icon: "K"
color: "#dc7d57"
sort: "title:asc"
default_fields:
  - source_refs
  - related_to
```

- `title` 优先作为类型名；未写时使用文件名。
- `icon` 和 `color` 用于知识图谱节点渲染。
- `sort` 和 `default_fields` 是类型定义元数据，第一版只作为只读图谱元信息暴露。

## 非目标

- 本协议不要求现有文件立即迁移。
- 本协议不要求普通稿件都写 frontmatter。
- 本协议不自动把知识卡写入创作记忆树。
- 本协议不自动把作品事实写入知识库。
- hot cache 只辅助对话召回最近常用知识卡，不替代用户显式 `@` 选择，也不作为长期事实来源。
- 冲突和不确定性判断由 `zhishiku-skill` 或用户确认产出；Wridian 主程序只读取字段、关系和 callout 做只读展示。
