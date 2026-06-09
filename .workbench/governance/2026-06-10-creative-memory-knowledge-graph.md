# 2026-06-10 Creative Memory And Knowledge Graph

## 背景

用户确认 Wridian 需要从概念上分开作品项目域和通用知识域：

- 左侧仍保留“作品库 / 知识库”两个标签。
- 作品库下显示作品项目和稿件。
- 作品项目内部“元素/设定”入口暂时不做。
- 记忆树改名为“创作记忆树”。
- `KNOWLEDGE.md` 不移出创作记忆树，中文名改为“知识调用”，内容改为知识库调用机制。
- 对话区顶部下拉只保留普通对话和作品项目文件夹名；选择作品项目时读取该项目压缩记忆。
- 对话输入 `@` 只选择知识库内容，先选分类文件夹，再选知识卡。
- 工作界面右上角在创作记忆树图标右侧增加知识图谱入口，弹窗尺寸与创作记忆树一致。

## 本轮变更

- 顶部“记忆树”入口和弹窗改为“创作记忆树”。
- 创作记忆树中 `KNOWLEDGE.md` 分支中文名改为“知识调用”，默认内容改为外部知识库/知识图谱调用机制。
- 每个作品项目记忆目录新增 `compressed.md` 压缩记忆文件；Project Mode 读取选中作品项目的压缩记忆。
- `@` 建议改为知识库分类优先：先显示含 Markdown 知识卡的分类文件夹，选中分类后再显示该分类下知识卡。
- 新增只读知识图谱后端命令，按当前用户选择的知识库 Markdown 分类、知识卡和 wikilink 生成节点与边。
- 顶部创作记忆树图标右侧新增知识图谱图标，点击打开与创作记忆树同尺寸的动态图谱弹窗。
- 更新项目地图，固定作品项目域和通用知识域的边界。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib` 通过，16 个单元测试通过，包含知识图谱目录/卡片/wikilink 生成测试。

## 回退依据

- 图谱入口集中在 `src/App.tsx`、`src/App.css` 和 `src-tauri/src/knowledge_graph.rs`，回退时移除命令注册和顶部按钮即可。
- Project Mode 压缩记忆读取集中在 `src-tauri/src/memory.rs` 和 `src-tauri/src/projects.rs`。
- `@` 分类选择集中在 `src/chat/promptContext.ts` 和 `src/App.tsx` 的知识建议索引。
