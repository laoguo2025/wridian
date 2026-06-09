# 2026-06-10 Library, Memory, Dialogue Context

## 背景

用户指出当前三类文件夹和对话机制存在问题：

- 作品库、知识库不应面向用户默认使用 AppData 绝对目录；用户应先选择本地文件夹。
- 记忆树不应镜像知识卡死副本，应从用户选择的知识库同步读取。
- 记忆树叶子需要在画布中以暖橙色小圆点展示并可点击编辑。
- 产品语义不再使用“共创对话”，统一为“对话”。
- 对话上下文机制需要参考 obsidian-copilot、claude-obsidian、OpenHuman、holaOS、Hermes、OpenClaw、SillyTavern 等项目。

## 参考调研

- 本地 obsidian-copilot 调研：采用消息显示文本与模型上下文 envelope 分离，L1-L5 分层，pill 是引用选择器，发送时读取快照。
- 本地 claude-obsidian 调研：采用 hot.md、index、具体页面的分层读取；原始资料和结构化 wiki 分离。
- 本地 OpenHuman 调研：PROFILE/MEMORY/USER、树摘要、用户反思和人格记忆分层，并有字符预算。
- 本地 Hermes 调研：MEMORY/USER/SOUL 分离，强调 session prompt caching 和容量限制。
- 本地 OpenClaw 调研：上下文压缩后保留可取回引用 id，适合作为 Wridian 后续压缩/召回方向。
- 远程 holaOS 仓库说明：agent 不从零开始，会带回工作上下文和相关记忆；Wridian 对应采用用户选择库、记忆树和对话上下文分层，而不是固定 AppData 默认库。
- 远程 SillyTavern World Info 文档：额外信息可按条件插入 prompt，条目的正文才进入上下文；Wridian 对应采用显式 pill、记忆树命中和作品作用域读取，不默认全量注入。
- 本轮曾尝试浅克隆 holaOS 与 SillyTavern 到 `.workbench/runtime/`，网络超时且未取得源码提交；空的部分克隆目录已清理，未作为本轮代码依据。

## 本轮变更

- 后端工作区配置扩展为 `activeWorkRoot` 和 `knowledgeRoot` 双根。
- 前端文件树节点保留绝对路径用于本机读写，同时新增 `relativePath` 和 `library` 作为 UI/对话引用。
- 前端未选择作品库或知识库时显示选择文件夹空态，并禁用新建/打开默认库操作。
- 知识库 Markdown 从当前用户选择的知识库实时读取为记忆树知识叶，不再复制到 `leaves/knowledge/cards/`。
- 记忆树叶子在画布中渲染为 `#dc7d57` 小圆点，围绕分支标签显示，悬浮提示文件名，点击使用同样编辑窗编辑。
- 对话请求新增显式 `contextItems`，pill 内容不再写进 `selectedText`。
- 用户可见文案从“共创”调整为“对话”。

## 验证

- `cargo test --manifest-path src-tauri\Cargo.toml --lib` 通过，9 个单元测试覆盖相对路径、知识卡实时读取、默认库未选择不接入记忆树、pill 显式上下文和默认库上下文拒绝。
- `cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- `npm run build` 通过。
- `rg -n "共创" src src-tauri .workbench/doc .workbench/reademe.md -S` 无命中。
- 浏览器打开 `http://127.0.0.1:1420` 静态验证：未选择库时显示选择文件夹空态，新建/打开按钮禁用；记忆树弹窗结构正常，分支文案为“创作旅程”。

## 未覆盖边界

- Vite 浏览器环境没有 Tauri `invoke`，因此本轮浏览器只能验证静态结构；Tauri 命令链路由 Rust 测试和 TypeScript 构建覆盖。
- 外部缺失的 holaOS、SillyTavern 主仓库源码未在本机找到；本轮只采用本地调研结果和已知机制方向，没有把外部仓库作为产品依赖。

## 回退依据

- 路径选择变更集中在 `src-tauri/src/workspace.rs` 和前端文件栏；回退时恢复单根配置和默认 `read_work_tree` 即可。
- 知识卡实时读取集中在 `src-tauri/src/memory.rs` 的 `knowledge_cards_folder_node` 和 `collect_knowledge_card_nodes`；回退时恢复旧镜像迁移函数即可。
- 对话上下文拆分集中在 `src/chat/cocreationClient.ts`、`src/chat/chatManager.ts`、`src/chat/messageRepository.ts` 和 `src-tauri/src/cocreation.rs`。
