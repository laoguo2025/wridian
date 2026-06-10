# 知识图谱滚轮缩放黑屏修复

## 现象

用户反馈知识图谱弹窗内使用鼠标滚轮缩放后，Wridian 窗口变成黑屏，画布不再显示。

## 排查

- 入口在 `src/App.tsx` 的 `KnowledgeGraphDrawer`。
- 滚轮缩放会频繁更新 SVG `<g>` 的 `transform` 属性。
- 同一图谱内部此前还通过 CSS keyframes 对 SVG 分组和节点 `<g>` 执行 `transform` 动画。
- 该组合在 WebView2/SVG 渲染链路里风险较高，容易与高频滚轮缩放叠加出渲染失效。

## 变更

- 移除知识图谱 SVG 分组和节点 `<g>` 上的 CSS `transform` 动画。
- 保留连线 `stroke-dashoffset` 流动效果。
- 将节点动效改为圆点透明度和描边宽度的轻微呼吸，不再改写 SVG transform。
- 为图谱 viewport 增加有限值、缩放范围和位移范围保护，滚轮输入异常时回落到安全视口。

## 验证

- `npm run build` 通过。
- `rg` 确认知识图谱相关旧 `graph-breathe`、`graph-node-pulse` 和 SVG 节点缩放动效已移除。

## 回退

回退本次 `src/App.tsx` 与 `src/App.css` 修改即可恢复旧缩放和动效行为。若只需要回退视觉动效，可仅恢复 CSS 中 `graph-motion` 和 `graph-node` 的 transform keyframes。
