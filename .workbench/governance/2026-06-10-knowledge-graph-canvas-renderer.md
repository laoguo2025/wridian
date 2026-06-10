# 知识图谱 Canvas 渲染器修复

## 现象

用户在测试版中继续复现：知识图谱弹窗内滚轮放大/缩小时，Wridian WebView2 整窗变黑，画布消失。

## 复核

- 上一轮只移除了 SVG transform 动效，但仍保留 SVG `<g>` 视口缩放。
- 复现说明 WebView2 对该图谱的高频 SVG group transform 仍不稳定。
- 重新核对 OpenHuman：
  - `PixiGraph.tsx` 只做 React 生命周期宿主。
  - `pixiGraphRenderer.ts` 使用单 canvas/WebGL renderer 承载图谱。
  - 滚轮缩放只更新 renderer 内部 world scale/position，并保持鼠标下图谱点不动。
  - 节点拖拽、画布拖拽、hover 和 open 都走 renderer hit-test，不依赖每个节点 DOM。

## 变更

- Wridian 知识图谱从 SVG 节点树改为单 Canvas 2D 渲染器。
- 保留现有 React 数据、预览和打开文件流程。
- 滚轮缩放改为 canvas camera：`scale + offsetX + offsetY`。
- 画布拖拽直接更新 camera offset。
- 节点拖拽和 hover 通过图谱坐标 hit-test 完成。
- 点击知识卡节点仍打开文件；点击文件夹节点不打开。
- 移除旧 `graph-node`、`graph-edge`、`graph-motion` SVG 样式和 keyframes。

## 验证

- `npm run build` 通过。
- `git diff --check` 通过。
- `rg` 确认旧 SVG 图谱 class 无残留，知识图谱渲染入口只剩 canvas。

## 回退

回退 `src/App.tsx` 和 `src/App.css` 中本次 Canvas renderer 改动即可恢复上一版 SVG 图谱。若 WebView2 继续出现图谱渲染层问题，下一步应完整引入 OpenHuman 的 Pixi/WebGL renderer，而不是回到 SVG group transform。
