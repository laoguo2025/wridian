# 2026-06-09 memory tree base image

## 变更原因

用户提供 `C:\Users\Administrator\Desktop\ai-image-1781008579118.png`，要求记忆树弹窗使用该图作为底图，继续保持工作界面左侧文件树样式不变。

## 变更范围

- 仅调整记忆树弹窗内的仿真树视觉。
- 新增项目内资源 `src/assets/memory-tree-base.png`。
- 将原代码绘制的 SVG 树骨架替换为底图图片，并缩小主干、分支和叶子卡片。

## 验证

- `npm run build` 通过。
- `cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- 检查 PNG 四角 alpha 为 0，已从假棋盘背景处理为真实透明背景。

## 回退

如需回退，移除 `src/assets/memory-tree-base.png`，恢复 `MemoryTreeSkeleton` 组件和对应 `.memory-tree-skeleton` / `.memory-tree-spine` / `.memory-tree-branch` CSS。
