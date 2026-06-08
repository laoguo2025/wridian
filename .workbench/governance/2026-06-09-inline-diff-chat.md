# 2026-06-09 Inline Diff Chat

## 背景

用户明确要求 Wridian 的共创能力不是“建议卡 + 插入按钮”，而是正常聊天/讨论，并能对选区或当前文件执行修改。修改必须直接在正文中以 diff 效果显示，红色为删除、绿色为新增，提供整体和局部确认/取消。

用户同时明确：正文区只有编辑模式，不需要 Markdown 预览或审阅模式；Wridian 面向小说作者和短剧编剧，默认不依赖 Markdown 格式效果。

## 变更

- 后端 `wridian_cocreate` 改为返回 JSON：
  - `reply`：正常聊天回复。
  - `edits`：可选正文替换建议，包含精确 `target`、`replacement` 和 `rationale`。
- 前端共创侧边面板改成消息流：
  - 用户消息支持编辑、复制、添加到记忆。
  - Wridian 回复支持重试、复制、添加到记忆。
- 正文编辑区改为纯文本 `contenteditable` 编辑器：
  - 支持选择片段并添加到输入框。
  - 有待确认修改时，直接在正文流里显示 inline diff。
  - 文件顶部显示全部确认/全部取消。
  - 每处修改旁显示确认/取消。
- 记忆提取按钮移动到文件顶部。

## 验证

- `npm run build` 通过。
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` 通过。
- 使用 VS Build Tools 环境运行 `cargo check` 通过。
- 使用 VS Build Tools 环境运行 `cargo test --lib` 通过：5 passed，1 ignored。

## 后续

- 需要真实桌面端手动验收 inline diff 的可读性、选区偏移和批量修改确认顺序。
- 需要后续补更稳的多处相同文本匹配策略，避免模型返回重复 target 时定位不清。
