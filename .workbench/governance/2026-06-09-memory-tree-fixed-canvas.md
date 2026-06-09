# 2026-06-09 Memory Tree Fixed Canvas

## 背景

用户反馈记忆树弹窗内部出现左右滚动条，要求固定画布大小，不要出现左右或上下滚动条。

## 变更

- 记忆树弹窗固定为不超过视口的 900x736 画布。
- 记忆树内部 `.memory-forest` 改为填满父容器并隐藏溢出。
- 移除内部画布的最小宽度和 padding，避免树图加内边距撑出横向滚动条。
- 弹窗容器补 `min-height: 0`，避免 flex 子项被内容撑出纵向滚动。

## 验证

- `npm run build` 通过。
- `cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- 内置浏览器打开 `http://localhost:1420` 后检查弹窗 DOM 尺寸：
  - `.memory-forest-shell` 的 `scrollWidth == clientWidth`，`scrollHeight == clientHeight`。
  - `.memory-forest` 的 `overflowX/overflowY` 为 `hidden`，且 `scrollWidth == clientWidth`，`scrollHeight == clientHeight`。

## 回退

回退 `src/App.css` 中 `.memory-tree-drawer`、`.memory-forest` 和通用抽屉 `min-height` 相关改动即可。
