# 2026-06-10 Knowledge Graph Frontmatter Relations

## 背景

用户要求先走 Tolaria 方向的轻量边界层，不做 SillyTavern 式作品设定和插入规则。

## 变更

- 知识图谱扫描知识卡 frontmatter，任何字段值中包含 `[[wikilink]]` 都会生成 `frontmatter:<field>` 关系边。
- 正文 wikilink 仍生成 `wikilink` 边；文件夹包含关系仍生成 `contains` 边。
- 图谱画布中 frontmatter 关系使用更清晰的实线，并在连线中点显示字段名短标签。

## 非目标

- 未做候选箱 / Inbox。
- 未做改名或移动后的 wikilink 自动修复。
- 未改变作品域和知识域的归属边界。

## 验证

- 待跑 Rust 定向测试和前端类型检查。
