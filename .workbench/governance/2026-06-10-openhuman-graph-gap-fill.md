# 2026-06-10 OpenHuman Graph Gap Fill

## 背景

用户要求对照 OpenHuman 图谱代码：已有相同功能的核对代码并查缺补漏；没有的功能源码级复刻，但适应 Wridian。

## OpenHuman 对照

- `MemoryGraph.tsx`：SVG fallback 中包含背景拖拽、节点拖拽、鼠标位置缩放、Reset view、hover 预览、拖拽不误打开。
- `pixiGraphRenderer.ts`：Pixi/WebGL + d3-force 路径包含同样的拖拽、缩放、hover、打开和 reset 行为。
- `memoryGraphLayout.ts`：图谱布局抽出 palette、半径、hit test、force layout 和缩放边界。

## 本轮适配

- 已有功能补细节：
  - 滚轮缩放改为围绕鼠标位置缩放，而不是只改中心 scale。
  - 拖拽手势继续抑制点击打开，避免误触。
- 新补功能：
  - 自动 fit-to-view 初始视图。
  - “重置视图”按钮。
  - 鼠标悬浮知识卡预览。
  - 按住节点可拖动节点位置；按住背景仍拖动画布。
- 未搬内容：
  - 未引入 Pixi/WebGL 和 d3-force 依赖；当前知识图谱节点量较小，继续用 Wridian 现有 SVG。
  - 未搬 OpenHuman 的中心性、PageRank、memory intelligence 面板；这些属于后续知识域分析能力，不是本轮图谱交互修补。

## 验证

- `npm run build` 通过。

## 回退依据

- 交互集中在 `src/App.tsx` 的 `KnowledgeGraphDrawer` 和 `fit/zoom` helper。
- 悬浮预览样式集中在 `src/App.css` 的 `.knowledge-graph-preview*`。
