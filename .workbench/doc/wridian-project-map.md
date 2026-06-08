# Wridian Project Map

## 定位

Wridian 是独立桌面写作共创系统，当前优先级是本地写作文件、写作记忆、共创对话和简化模型接入。

Wridian 不只用于写小说，也用于短剧剧本、剧本、分集大纲、场景稿、人物小传和设定资料。产品定位是“带写作记忆的本地写作共创系统”，不是通用 AI OS、知识库问答壳、模型供应商管理器或只服务长篇小说的编辑器。

第一屏应保持极简：作品文件区、稿件编辑区、右侧对话区。记忆、模型、人物、设定、历史和抽取动作进入侧边面板或设置，不抢占稿件编辑区。

## 当前入口

- 前端入口：`src/App.tsx`
- 右侧对话区：`src/chat/ChatPanel.tsx`
- 聊天运行管理：`src/chat/chatManager.ts`
- 聊天持久化：前端 `src/chat/chatPersistence.ts`，后端 `src-tauri/src/chat_persistence.rs`
- 聊天输入组件：`src/chat/CopilotPromptEditor.tsx`
- 聊天上下文：`src/chat/promptContext.ts`
- 聊天消息仓库：`src/chat/messageRepository.ts`
- 共创请求客户端：`src/chat/cocreationClient.ts`
- 正文替换保护：`src/editor/draftReplaceGuard.ts`
- 主要样式：`src/App.css`
- Tauri 组装入口：`src-tauri/src/lib.rs`
- 后端模块：
  - `src-tauri/src/runtime.rs`：本地数据目录、默认 Vault、运行时文件路径。
  - `src-tauri/src/workspace.rs`：本地作品目录、文件树、正文读写。
  - `src-tauri/src/model_accounts.rs`：自定义 OpenAI-compatible API 配置和测试。
  - `src-tauri/src/memory.rs`：待确认记忆和写入记忆。
  - `src-tauri/src/cocreation.rs`：共创请求上下文组装和模型回复。
- 本地运行：`npm run dev`
- Rust 检查：普通 PowerShell 中可直接运行 `cargo check --manifest-path src-tauri\Cargo.toml`；项目通过 `.cargo/config.toml` 注入本机 MSVC Build Tools 路径，避免每次手工执行 `vcvars64.bat`。

## 当前边界

- 本地文件只支持 `md`、`markdown`、`txt`、`fountain`。
- 文件读写只允许默认 Vault 或用户选择的工作目录内文件。
- 文件区采用 Obsidian 式结构：顶部新建文件/文件夹/作品文件夹，树节点支持多层级展开/收回和右键菜单，底部系统设置。
- 文件区“移到回收站”只移动到当前工作根目录 `.wridian-trash/`，不做永久删除。
- 模型接入先支持一个 OpenAI-compatible 自定义 API。
- 记忆 MVP 使用 `.wridian/memory-tree.json` 和 `.wridian/candidates.json`；模型只提取待确认候选，用户编辑确认后才写入长期记忆。
- 暂不接入生图、生视频和复杂模型网关。

## 交互边界

- 稿件编辑区只负责当前文件内容，不承载聊天历史，不因共创回复而挤占正文。
- 软件启动后不默认展示示例作品；未选择文件时稿件编辑区为空背景，只在中间显示“文件编辑区”。
- 稿件编辑区始终是纯文本编辑器，不做 Markdown 预览或独立审阅模式；小说作者和短剧编剧默认不依赖 Markdown 格式效果。
- 正文长度只允许稿件编辑区内部滚动，不允许撑出整个工作界面的窗口级上下滚动条。
- 底部共创输入应进入共创流程，不得直接创建记忆候选或自动打开记忆抽屉。
- 共创对话区常驻在工作界面右侧，按 `obsidian-copilot` 的简洁侧栏聊天形态复刻可见交互：消息流为空时不显示说明卡片，输入框位于对话区底部，发送后只更新右侧消息流，不弹出共创抽屉。
- 当前对齐的 `obsidian-copilot` 源码基线：
  - `ChatInput.tsx`：带边框的底部输入容器、上下文 pill 区、中间约 60px 起步输入区、24px 底部工具栏、小发送/停止动作。
  - `LexicalEditor.tsx`：输入区内部滚动，长文本不撑高右栏；Wridian 聊天输入区已从 textarea 切换为 Lexical `ContentEditable`，使用受控文本同步、历史插件和 Enter 发送；实现入口为 `src/chat/CopilotPromptEditor.tsx`。
  - `AtMentionCommandPlugin.tsx` / `SlashCommandPlugin.tsx`：Wridian 已接入本地第一版 `@` 上下文选择和 `/` 写作命令提示，实现在 `src/chat/CopilotPromptEditor.tsx` 内。`@` 可把当前选区、当前文件、当前正文放入输入框上方上下文 pill；`/` 可插入改对白、增强冲突、加结尾钩子、检查角色口吻、批量改角色名、提取记忆等小说和短剧共用命令。
  - `ContextManager.ts` / `PromptContextTypes.ts`：Wridian 已开始拆出聊天上下文边界，`src/chat/promptContext.ts` 负责 prompt pill 类型、序列化、上下文建议构造和写作命令建议；消息仓库只保存消息和已绑定的上下文快照。
  - `MessageRepository.ts`：Wridian 已开始拆出前端消息仓库边界，`src/chat/messageRepository.ts` 负责消息类型、ID、用户/助手消息创建、编辑恢复和重试定位；`App.tsx` 仍负责调用 Tauri 共创命令。
  - `ChatManager.ts` / `ChatPersistenceManager.ts`：Wridian 已引入本地前端版 `src/chat/chatManager.ts`，负责消息列表、pending/error、发送共创请求、追加助手回复和生成待确认正文修改；聊天记录通过 `src/chat/chatPersistence.ts` 调用后端 `src-tauri/src/chat_persistence.rs` 保存为 `.wridian/chat/<session>.md`。
  - `ChatMessages.tsx`：空消息流保持空白，Relevant Notes / Suggested Prompts 这类辅助块不固定展示；Wridian 右侧消息流和输入组合入口为 `src/chat/ChatPanel.tsx`。
  - `ChatSingleMessage.tsx` / `ChatButtons.tsx`：用户消息使用浅边框背景，AI 消息不做重卡片；消息动作放在底部紧凑行。
  - pill 节点：Wridian 已建立本地 `PromptContextPillKind` 数据结构，覆盖 selection、active-file、file、url、tool、memory，并在右侧输入区和消息上下文中按类型渲染；当前仍是 React pill，不是 Lexical DecoratorNode。
  - 后续仍需补齐 Copilot 的完整文件内容检索、URL/工具 pill 插入、模型/工具选择器、图片粘贴和真正 Lexical 自定义节点；当前 `@`/`/` 是本地 MVP，不是最终完整 Copilot 插件边界。
- 记忆命中、注入和上下文选择默认在后台执行，不在右侧对话区常驻展示“本次使用的记忆”等系统说明；记忆面板只由顶部“记忆”、显式“从当前正文提取”或“记住这条”动作打开。
- 右侧侧边面板应支持模式切换，第一版至少区分“共创”和“记忆”。
- 记忆提取是显式动作；模型不得在普通共创发送时直接写长期记忆。
- 当前已完成最小共创/记忆分离：底部输入调用共创命令并打开“共创”侧边面板，不再创建候选记忆；顶部“记忆”按钮仍打开记忆面板。
- 正文 inline diff 的确认链路参考 `obsidian-copilot` 的 `replaceGuard.ts`，已接入纯文本替换保护：只有 target 在当前正文中唯一命中且不与其他修改范围重叠时才渲染和确认；找不到、重复出现或重叠的建议会保持待确认状态并提示需要重新定位，禁止默认改第一处。

## 作品类型

- 小说模式：章节、场景、人物、世界观、剧情线、伏笔、禁区、风格。
- 短剧/剧本模式：集、场、对白、转折、冲突、钩子、角色口吻、场地/预算限制、分集节奏。
- `.fountain` 不是普通可打开文件类型，后续应升级为剧本工作流：场景识别、角色对白、outline、预览和导出。
- UI 文案避免过早写死为“章节”；默认可用“稿件”“当前文件”“作品文件”，需要时按文件类型显示“章节 / 场景 / 剧本段落”。

## 借鉴边界

- `obsidian-copilot`：只借交互模型。正文稳定，AI 在侧栏/命令中辅助；参考 Vault QA、Relevant Notes、选中文本命令、Project Mode 和上下文选择。
- `claude-obsidian`：借 Markdown 知识图谱和 ingest 方法。作品稿件作为 sources，人物/地点/物件作为 entities，主题/风格/规则作为 concepts，使用交叉引用、索引和 hot context 辅助召回。
- `OpenHuman`：只借 Memory Tree、Markdown vault、本地优先桌面结构；不借托管登录、OAuth、复杂集成、搜索代理和通用个人 AI OS 方向。
- `holaOS`：借 continuity 分层。当前写作现场、长期写作记忆、工作区/作品规则必须分开。
- `SillyTavern`：借 World Info、角色卡和插入规则，用在人物、设定、伏笔、禁区和风格条目上；不借角色聊天/RP 产品形态。
- `Beat`、`Better Fountain`、`Fountain`：借剧本纯文本格式、场景识别、outline、极简剧本编辑体验、预览和导出思路。
- `Twine`、`ink`、`Yarn Spinner`：后续借分支剧情、故事状态、节点式对白和条件逻辑，不作为 MVP。
- `Basic Memory`、`Graphiti`、`Pi`：后续分别参考 Markdown 语义图谱、时序事实/冲突检测、session tree/continue/fork；不进入最小 MVP。

## 记忆存储

- 记忆文件夹：Wridian 数据目录下的 `.wridian/`。
- 聊天记录：`.wridian/chat/*.md`，每个运行会话保存为 Markdown，包含 frontmatter、来源文件、用户/助手消息和上下文 pill。
- 长期记忆：`.wridian/memory-tree.json`。
- 候选记忆：`.wridian/candidates.json`。
- 记忆条目支持写作分类：人物、世界观、剧情线、风格、禁区、其他。
- 模型提取不得直接写入长期记忆，必须经过候选、编辑、确认。

## 最小 MVP 路线

目标：先完成“本地稿件编辑 + 共创回答 + 显式记忆提取/确认 + 记忆注入”的闭环。

1. 修正共创/记忆交互混用。
   - 底部输入框发送后走共创流程，不再调用候选记忆创建，不再自动打开记忆抽屉。
   - 增加“共创”侧边面板显示 AI 回复和可执行建议。
   - 保留“记忆”侧边面板，记忆提取只由显式按钮触发。
2. 实现最小共创请求。
   - 组装当前文件、选中文本或当前正文片段、active context、已确认相关记忆和用户输入。
   - 调用已配置的 OpenAI-compatible API，返回写作建议。
   - 第一版只展示回复，不自动改正文。
3. 做回复到正文的安全操作。
   - 支持用户选中正文片段并添加到输入框。
   - 共创回复底部支持重试、复制、添加到记忆；用户消息底部支持编辑、复制、添加到记忆。
   - Wridian 对正文的修改以正文内联 diff 展示，红色为删除、绿色为新增。
   - 文件顶部提供全部确认和全部取消；每处修改提供确认和取消。
   - 不做自动全文覆盖。
   - 当前已完成：正文区使用纯文本编辑器承载 inline diff，确认后写入正文并继续走已有自动保存链路；inline diff 只是编辑器内的待确认建议，不是单独审阅模式。
4. 完成记忆注入闭环。
   - 已确认记忆按作品/文件/分类筛选，参与共创上下文。
   - 普通共创不直接写记忆；可从回复中显式“记住这条”进入候选。
5. 补剧本 MVP。
   - 对 `.fountain` 或剧本稿件显示剧本上下文提示。
   - 共创命令支持改对白、增强冲突、加结尾钩子、检查角色口吻、拆分分集节奏。
6. 后续增强。
   - Project Mode、Relevant Notes、实体/概念/来源拆分、Memory Tree 可视化、分支续接、时序冲突检测。

## 后端约束

- `src-tauri/src/lib.rs` 只负责模块声明、插件挂载和命令注册。
- 新业务逻辑必须进入对应模块；没有对应模块时先建小模块，不继续膨胀 `lib.rs`。
