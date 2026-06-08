# 2026-06-09 Editor Chat Layout

## 背景

用户要求启动后不要展示示例作品；工作界面右侧增加常驻对话区，并把共创输入框放到对话区底部；正文长度不能让整个窗口出现上下滚动条，滚动只属于文件编辑区。

用户进一步确认：输入框发送消息后不是弹出抽屉，而是在正常对话区显示消息。

## 变更

- 删除前端默认示例作品、示例正文、示例记忆和示例共创分支。
- 初始状态改为空文件编辑区，中间只显示“文件编辑区”小字。
- 将共创抽屉改为右侧常驻 `ChatPanel`，消息流、记忆摘要和输入框都在右栏内。
- 发送消息不再打开共创抽屉，只更新右侧对话区；记忆抽屉仍由显式记忆动作打开。
- 按 obsidian-copilot 式侧栏聊天收敛 UI：删除空状态说明、当前文件提示、记忆注入展示卡片；右侧只保留消息流和底部输入框。
- 输入框压缩为约三行高度，内部滚动承载更长文本，发送按钮缩小。
- 已实际查看 `obsidian-copilot` 源码：
  - `src/components/chat-components/ChatInput.tsx`：输入区由上下文控制、Lexical 输入、底部工具栏组成。
  - `src/components/chat-components/LexicalEditor.tsx`：输入区 `min-height` 约 60px，内部滚动，支持 `@`、`/`、pill 和粘贴。
  - `src/components/chat-components/ChatMessages.tsx`：空消息时辅助组件可由设置开关控制，不是固定说明面板。
  - `src/components/chat-components/ChatSingleMessage.tsx`：用户消息和 AI 消息底部提供动作按钮。
- 追加修正：根据用户反馈，“复刻”必须基于 `obsidian-copilot` 实际代码而不是泛泛参考。本轮重新按源码对齐可见形态：
  - 右侧消息流改为 `ChatMessages` / `ChatSingleMessage` 风格的整宽流；用户消息浅边框背景，AI 消息不做显眼重卡片。
  - 移除消息顶部角色标签和大块选区引用；选区上下文改为类似 context badge 的小 pill。
  - 消息动作改为底部紧凑行，保留用户要求的用户消息“编辑、复制、记忆”和 AI 消息“重试、复制、记忆”。
  - 输入容器继续按 `ChatInput` / `LexicalEditor` 的可见结构：边框容器、pill 区、约 60px 输入区、24px 底部工具栏、小发送按钮；长文本在输入区内部滚动。
  - 暂不引入 Lexical、完整 `@`/`/` 类型提示、图片 pill、工具开关和模型选择；这些属于后续功能，不再标记为已复刻。
- 根页面和工作区固定视口高度，隐藏窗口级滚动；正文编辑器、文件树、右侧聊天消息区使用内部滚动。
- 增加主题化滚动条样式。

## 验证

- `npm run build` 通过。
- `cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- Vite 预览截图确认：启动后没有示例正文，中间为空编辑区小字，右侧对话区常驻，输入框位于右栏底部。
- 尝试 5178 端口时系统拒绝监听，改用 3000 端口完成截图验证。

## 回退

回退 `src/App.tsx` 与 `src/App.css` 中的布局改动即可恢复旧的示例作品和共创抽屉；不涉及后端数据结构变更。
