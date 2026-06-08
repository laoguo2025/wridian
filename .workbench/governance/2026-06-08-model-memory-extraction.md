# 2026-06-08 Model Memory Extraction

## 目标

- 从当前正文调用已配置第三方 API 提取候选记忆。
- 提取结果只进入待确认候选，不直接写入长期记忆。

## 变更

- 新增 `wridian_extract_memory_candidates` 命令。
- 自定义 API 配置由记忆模块复用，不新增模型网关。
- 模型输出按 JSON 解析为人物、世界观、剧情线、风格、禁区、其他。
- 记忆面板增加“从当前正文提取”按钮和分类标签。
- Runtime ID 改为纳秒级，避免一次生成多条候选时 ID 冲突。

## 验证

- 已通过：解析模型候选记忆的 Rust 单测。
- 已通过：读取 Chat Completions `choices[0].message.content` 的 Rust 单测。
- 已通过：`cargo fmt --check`
- 已通过：`cargo check`
- 已通过：`cargo test --lib`
- 已通过：`npm run build`
- 已通过：第三方 API 真实提取链路，返回可解析候选记忆。
