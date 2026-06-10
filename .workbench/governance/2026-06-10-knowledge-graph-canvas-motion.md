# 知识图谱 Canvas 动效补回

## 目标

Canvas 图谱替换 SVG 后，补回动态视觉，但不能回到 WebView2 不稳定的 SVG transform / CSS keyframes 路径。

## 变更

- 在知识图谱打开且有节点时启动 `requestAnimationFrame` 绘制循环。
- 连线使用 Canvas `lineDashOffset` 做流动效果。
- 节点使用时间相位改变半径、描边和光晕透明度，形成轻微呼吸效果。
- hover 节点保留更强光晕和描边反馈。

## 验证

- `npm run build` 通过。
- `git diff --check` 通过。

## 回退

回退 `src/App.tsx` 中本次动画循环和 `drawKnowledgeGraphCanvas` 的 time/pulse/dashOffset 改动即可保留静态 Canvas 图谱。
