# 2026-06-09 Memory Tree Vertical Offset

## 背景

用户要求记忆树底图和标签整体向下移动，让树根底部边缘落在灰色阴影中的标记位置。

## 变更

- 在记忆树画布上新增统一的 `--memory-tree-offset-y` 偏移量。
- 底图、自我意识标签、主干标签和八个分支标签全部使用同一 Y 方向偏移。
- 当前偏移为 24px，保持树和标签的相对位置不变。

## 验证

- `npm run build` 通过。
- `cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- 内置浏览器截图确认树和标签整体下移；浏览器预览会因为缺少 Tauri invoke 多出错误提示并挤压画布，桌面端无该提示。

## 回退

删除或调整 `--memory-tree-offset-y` 即可恢复或微调树和标签的整体垂直位置。
