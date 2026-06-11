# 2026-06-11 Chat Abort / Context Snapshot / Relevant Notes

## 目标

- 右侧对话发送后提供真实停止能力。
- 发送瞬间冻结输入、选区和上下文 pill，不依赖后续异步 state 更新。
- Relevant Notes 接入作品稿件与知识卡召回，并显示相关理由。

## 变更依据

- 现有对话请求由前端 `chatManager` 调用后端 `wridian_cocreate`，停止按钮此前只是 pending 禁用态。
- 现有消息仓库已能保存 `contextPills`，适合承载发送瞬间快照。
- 现有 `projects.rs` 已有 Relevant Notes 的词项重合与 wikilink/backlink 评分，可在同一入口扩展知识卡和理由字段。

## 本轮实现

- 每次发送生成 requestId，前端 stop 调用后端 abort 命令；后端模型等待路径可被取消，取消后不追加助手消息、不生成正文修改、不写长期记忆。
- App 层新增 prompt pill ref，pill 写入和发送快照同步更新，发送时冻结当前输入、pill、当前稿件和选区。
- Relevant Notes 扫描作品库与知识库 Markdown，返回稿件/知识卡类型、摘要和“同词 / 反链 / 共同链接 / 共同概念 / 共同来源”理由。
- 右侧对话区新增相关内容列表，点击后沿既有 file pill 注入链路进入上下文。

## 回退依据

- 前端回退点：移除 ChatPanel 相关内容区、App 的 `findRelevantNotes` effect、chatManager stop/requestId 接入。
- 后端回退点：移除 `wridian_abort_cocreate` 命令、requestId 取消注册表和 RelevantNote 扩展字段。

## 验证

- `npx tsc --noEmit` 通过。
- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib` 通过，37 个单元测试通过。
