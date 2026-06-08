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
- 进一步修正：
  - 正文区继续按纯文本编辑器处理，去掉前端代码和样式中的 review 命名，避免误导为独立审阅模式。
  - “全部确认”改为一次性匹配所有待确认修改，并按正文位置从后往前应用，避免 React 状态批处理导致只按旧正文循环替换。
  - 相同 target 的多处修改按正文中未占用位置顺序分配，支持不带选区的批量改名等跨段落修改。

## 验证

- `npm run build` 通过。
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` 通过。
- 使用 VS Build Tools 环境运行 `cargo check` 通过。
- 使用 VS Build Tools 环境运行 `cargo test --lib` 通过：5 passed，1 ignored。
- 本轮追加验证：
  - `npm run build` 通过。
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check` 通过。
  - 使用 `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat` 初始化后运行 `cargo check --manifest-path src-tauri\Cargo.toml` 通过。

## 后续

- 需要真实桌面端手动验收 inline diff 的可读性、选区偏移和批量修改确认顺序。
- 仍需真实桌面端手动验收带 inline diff 时的可读性，以及确认/取消后自动保存是否符合作者预期。
