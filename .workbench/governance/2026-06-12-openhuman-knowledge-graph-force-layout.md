# 2026-06-12 OpenHuman Knowledge Graph Force Layout

## 背景

用户要求继续对标 OpenHuman 优化 Wridian 知识图谱。

## 借鉴边界

- 只借 OpenHuman 图谱的工程思路：d3-force 力导向布局、单 Canvas 渲染、自动 fit、鼠标位置缩放、拖拽画布、拖拽节点、hover 预览和 reset view。
- 不引入 OpenHuman 的托管来源、Memory Workspace、联系人图谱、中心性面板、PageRank 或通用 AI OS 能力。
- 不直接复制 OpenHuman 源码；Wridian 保留现有知识库 Metadata Index、知识卡、断链和 frontmatter typed relation 语义。

## 改动

- `src/knowledge/KnowledgeGraphDrawer.tsx`
  - 引入 `d3-force` 作为显式依赖。
  - 图谱布局从 0-100 手写碰撞模型改为固定世界坐标中的 force simulation。
  - 节点上限从 180 提高到 1000，边上限从 260 提高到 2400。
  - 节点大小按分类、断链和关系度计算；边距离/强度按 contains、wikilink、embed、frontmatter、unresolved 区分。
  - 保留 Canvas 绘制、动效、hover 预览、节点点击打开文件和体检/检索链路。
  - 修正节点拖拽坐标边界，避免旧 0-100 坐标 clamp 把节点拖回角落。
  - 增加图谱节点/关系统计条和截断提示。
- `src/App.css`
  - 增加图谱统计条样式，Canvas 光标由组件状态控制。
- `package.json` / `package-lock.json`
  - 新增运行依赖 `d3-force` 和开发类型依赖 `@types/d3-force`。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_graph --lib` 通过，5 个知识图谱后端测试通过。

## 回退

回退本次提交即可恢复到手写 Canvas 布局。若只回退依赖，必须同时恢复 `KnowledgeGraphDrawer.tsx` 中的手写 `relaxKnowledgeGraphLayout`，否则前端无法构建。
