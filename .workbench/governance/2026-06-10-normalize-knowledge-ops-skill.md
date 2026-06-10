# Normalize Knowledge Ops Skill

## 背景

用户指出“知识库运维”不是技能管理面板里的按钮工具，而是对话框中通过 `/` 使用的普通 skill；不应在 Wridian 内额外硬编码本地体检逻辑、特殊发送分支或结果注入。

## 变更

- 移除技能管理抽屉里的“运行体检”按钮和结果展示。
- 删除 Wridian 内置知识库体检后端命令和相关类型。
- 删除知识库运维专用上下文构造逻辑；`知识库运维` 与其他技能一样，通过 `/` 生成普通 tool pill，并随消息发送进入原有对话链路。
- 允许只有 prompt pill、没有正文文本时发送消息；默认文本使用通用“请按已选择的技能执行。”，不区分具体 skill。

## 非变化

- 技能管理仍负责启用/停用技能和显示本机 `zhishiku-skill` 是否被识别。
- 对话、聊天归档、模型请求和 tool pill 展示继续使用原有链路。

## 验证

- 源码残留搜索：没有 `wridian_get_knowledge_health`、`运行体检`、`sendLocalToolPrompt` 等旧入口残留。
- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml`：30 个测试通过。
- `git diff --check`：通过，仅有 Windows 换行提示。
