# Knowledge Graph Writing Language

## 背景

用户反馈知识图谱中的“派生自”“编译来源”“证据”等词偏编程或不易理解，需要改成写作知识库语义。

## 变更

- 用户可见图谱文案统一改为写作语义：`编译来源` -> `素材出处`，`派生自` -> `提炼自`，`证据` -> `依据材料`。
- 治理视图继续只读提示，不改变 frontmatter 字段名、解析协议或兼容字段。
- 长期项目地图和 frontmatter 关系协议同步改写中文解释，保留 `source/derived_from/evidence/source_refs` 等字段名。

## 验证

- `npm run build` 通过。
- 已使用内置浏览器刷新 `http://127.0.0.1:1420/.workbench/runtime/wridian-kg-browser-fixture.html` 验收真实图谱组件：治理 tab 和质量闸门显示“素材出处”，体检动作显示“补出处”，canvas 连线标签显示“依据材料”“提炼自”“素材来源”，页面控制台无错误。

## 回退

回退本次文案改动即可恢复旧口径；由于未改字段名和解析逻辑，不涉及数据迁移。
