# Wridian Project Map

## 定位

Wridian 是独立桌面写作对话系统，当前优先级是本地写作文件、写作记忆、对话和简化模型接入。

Wridian 不只用于写小说，也用于短剧剧本、剧本、分集大纲、场景稿、人物小传和设定资料。产品定位是“带写作记忆的本地写作对话系统”，不是通用 AI OS、知识库问答壳、模型供应商管理器或只服务长篇小说的编辑器。

第一屏应保持极简：作品文件区、稿件编辑区、右侧对话区。记忆、模型、人物、设定、历史和抽取动作进入侧边面板或设置，不抢占稿件编辑区。

## 当前入口

- 前端入口：`src/App.tsx`
- 右侧对话区：`src/chat/ChatPanel.tsx`
- 正文编辑器：`src/editor/DraftEditor.tsx`
- 聊天运行管理：`src/chat/chatManager.ts`
- 聊天持久化：前端 `src/chat/chatPersistence.ts`，后端 `src-tauri/src/chat_persistence.rs`
- 聊天输入组件：`src/chat/CopilotPromptEditor.tsx`
- 聊天上下文：`src/chat/promptContext.ts`
- 聊天消息仓库：`src/chat/messageRepository.ts`
- 对话请求客户端：`src/chat/cocreationClient.ts`
- 左侧文件树和右键菜单：`src/files/FileTree.tsx`
- 工作区文件命令客户端：`src/workspace/workspaceClient.ts`
- 稿件类型识别：`src/editor/draftKind.ts`
- 正文替换保护：`src/editor/draftReplaceGuard.ts`
- 创作记忆树抽屉：`src/memory/MemoryDrawer.tsx`
- 知识图谱抽屉：`src/knowledge/KnowledgeGraphDrawer.tsx`
- 知识卡建议索引：`src/knowledge/knowledgeSuggestions.ts`
- 技能管理抽屉：`src/skills/CreativeSkillsDrawer.tsx`
- 模型设置对话框：`src/settings/ModelSettingsDialog.tsx`
- 前端 SVG 图标：`src/icons.tsx`
- 前端共享 Tauri 数据类型：`src/appTypes.ts`
- 主要样式：`src/App.css`
- Tauri 组装入口：`src-tauri/src/lib.rs`
- 后端模块：
  - `src-tauri/src/runtime.rs`：本地数据目录、默认 Vault、运行时文件路径。
  - `src-tauri/src/workspace.rs`：本地作品目录、文件树、正文读写。
  - `src-tauri/src/model_accounts.rs`：自定义 OpenAI-compatible API 配置和测试。
- `src-tauri/src/memory.rs`：文件化记忆树、作用域记忆和知识卡读取。
  - `src-tauri/src/cocreation.rs`：对话请求上下文组装和模型回复。
- 本地运行：`npm run dev`
- Rust 检查：普通 PowerShell 中优先运行 `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`；脚本会定位本机 Visual Studio Build Tools 并进入 MSVC 环境，仓库不再提交 `.cargo/config.toml` 这类本机绝对路径配置。

## 当前边界

- 本地文件只支持 `md`、`markdown`、`txt`、`fountain`。
- 作品库面向用户时必须先由用户选择本地文件夹；知识库安装后默认在本机 D 盘创建并启用 `Wridian知识库` 根目录，若无 D 盘则回落到 Wridian 本地数据目录。用户仍可手动改选其他知识库根目录。
- 前端文件树节点同时携带绝对路径和相对路径：绝对路径只用于 Tauri 本地安全校验和读写，相对路径用于 UI、聊天上下文引用和跨库引用语义。
- 文件区采用 Obsidian 式结构：顶部新建文件/文件夹/作品文件夹，树节点支持多层级展开/收回和右键菜单，底部系统设置。
- 左侧文件区分为“作品库 / 知识库”标签页：作品库放作品项目、章节、剧本、分集、场景稿；知识库放人物、地点、设定、世界观、风格、禁区和资料摘录等知识卡。默认知识库分类模板为 `00知识库治理`、`01原始资料`、`02拆解报告`、`03故事模型`、`04人物原型`、`05情节方程`、`06写作技法`、`07综合素材`、`08大神蒸馏`、`09文件归档`；新建默认知识库时，`00知识库治理` 默认包含 `治理说明.md` 和 `调用记录台账.md`，`08大神蒸馏` 默认包含 `大神索引.md` 和 `_安装记录.md`。这些只是初始分类，用户可以在文件树里增、改、删，知识库运维 skill 体检时应按实际目录修正。
- 文件区“移到回收站”只移动到当前工作根目录 `.wridian-trash/`，不做永久删除。
- 模型接入先支持一个 OpenAI-compatible 自定义 API；配置测试必须同时验证 HTTP 成功和响应文本可解析。API Key 只保存到本机系统凭据，`model-accounts.json` 只保存 `baseUrl`、`model` 和 `keyStored` 状态；模型账户弹窗必须提供清除本机凭据入口。对话响应解析兼容 chat completions 字符串、content parts 和 Responses 风格 `output_text`/`output` 文本。
- 记忆系统以 `.wridian/memory-tree/` 下的 Markdown 文件树为主入口；用户在“创作记忆树”抽屉直接查看和编辑全局层、伙伴层、作品层和知识调用机制文件。
- 知识、知识库、知识卡和知识图谱属于作品项目之外的通用知识积累；作品项目可显式引用知识卡，但知识卡不自动变成作品记忆。
- 暂不接入生图、生视频和复杂模型网关。

## 作品域与知识域

- Frontmatter 关系协议唯一 owner：`.workbench/doc/wridian-frontmatter-relation-protocol.md`。
- 作品域负责正在写的东西：作品项目、作品库、稿件、相关元素、创作记忆树、Project Mode、Relevant Notes、选区命令和上下文选择。
- 创作记忆树是作品项目的记忆与规则树；它保存作品连续性、项目规则、人物边界、禁区、当前进度和项目压缩记忆。
- 作品相关元素可以引用知识卡，但不拥有知识卡；引用知识卡不等于把知识卡写成作品记忆。
- 知识域负责作品项目之外的通用知识积累：知识、知识库、知识卡、知识图谱、ingest、sources、entities、concepts、交叉引用和 hot cache。
- 知识图谱是作品项目之外的通用知识网络；它服务多个作品，不归属于单个作品项目。
- 知识卡可以被多个作品引用，但不会自动变成任何作品的记忆。
- 从知识到作品只能通过“引用 / 采纳 / 改写成作品设定”进入项目。
- 从作品到知识只能通过“摘录 / 抽象 / 沉淀为知识卡”离开项目。
- `obsidian-copilot` 只借给作品域：Project Mode、Relevant Notes、选区命令、上下文选择。
- `claude-obsidian` 只借给知识域：ingest、sources/entities/concepts、交叉引用、hot cache。
- `tolaria` 借给作品域和知识域之间的边界：frontmatter 关系、capture/organize、文件系统唯一真相。
- 知识库不使用“未结构化文件候选箱”判定知识卡是否正式。`01原始资料` 本来就是未加工素材，`02拆解报告` 保存分析产物和 A/B/C 候选，只有通过 `zhishiku-skill` / `tilian-skill` 质量闸门的 S 级卡才能写入 `03-07` 成为正式知识卡。

## 交互边界

- 稿件编辑区只负责当前文件内容，不承载聊天历史，不因对话回复而挤占正文。
- 软件启动后不默认展示示例作品；未选择文件时稿件编辑区为空背景，只在中间显示“文件编辑区”。
- 稿件编辑区始终是纯文本编辑器，不做 Markdown 预览或独立审阅模式；小说作者和短剧编剧默认不依赖 Markdown 格式效果。
- 正文长度只允许稿件编辑区内部滚动，不允许撑出整个工作界面的窗口级上下滚动条。
- 底部对话输入应进入对话流程，不得直接创建记忆候选或自动打开记忆抽屉。
- 对话区常驻在工作界面右侧，按 `obsidian-copilot` 的简洁侧栏聊天形态复刻可见交互：消息流为空时不显示说明卡片，输入框位于对话区底部，发送后只更新右侧消息流，不弹出额外抽屉。
- 当前对齐的 `obsidian-copilot` 源码基线：
  - `ChatInput.tsx`：带边框的底部输入容器、上下文 pill 区、中间约 60px 起步输入区、24px 底部工具栏、小发送/停止动作。
  - `LexicalEditor.tsx`：输入区内部滚动，长文本不撑高右栏；Wridian 聊天输入区已从 textarea 切换为 Lexical `ContentEditable`，使用受控文本同步、历史插件和 Enter 发送；实现入口为 `src/chat/CopilotPromptEditor.tsx`。
  - `AtMentionCommandPlugin.tsx` / `SlashCommandPlugin.tsx`：Wridian 已接入本地第一版 `@` 知识卡选择和 `/` 技能调用提示，实现在 `src/chat/CopilotPromptEditor.tsx` 内。`@` 菜单只选择知识库内容，先显示知识库下含 Markdown 知识卡的分类文件夹，选中分类后再显示该分类下的知识卡；选中知识卡后读取文件内容并以 memory pill 注入上下文。`/` 菜单只显示“技能管理”中当前启用的技能。
    - 剧本模式：前端按 `.fountain` 扩展名、内景/外景/集/场信号和角色对白行识别短剧/剧本稿件；稿件类型会进入对话请求，`/` 菜单仍只显示“技能管理”中当前启用的技能。
  - 文件/上下文检索：右键文件或点击相关稿件仍可把文件内容作为 `file` pill 注入；`@` 菜单不再搜索作品文件，只搜索知识卡。
  - `ContextManager.ts` / `PromptContextTypes.ts`：Wridian 已开始拆出聊天上下文边界，`src/chat/promptContext.ts` 负责 prompt pill 类型、序列化、上下文建议构造和写作命令建议；消息仓库只保存消息和已绑定的上下文快照。
  - `MessageRepository.ts`：Wridian 已开始拆出前端消息仓库边界，`src/chat/messageRepository.ts` 负责消息类型、ID、用户/助手消息创建、编辑恢复和重试定位；`App.tsx` 仍负责调用 Tauri 对话命令。
  - `ChatManager.ts` / `ChatPersistenceManager.ts`：Wridian 已引入本地前端版 `src/chat/chatManager.ts`，负责消息列表、pending/error、发送对话请求、追加助手回复和生成待确认正文修改；聊天记录通过 `src/chat/chatPersistence.ts` 调用后端 `src-tauri/src/chat_persistence.rs` 保存为 `.wridian/chat/<session>.md`。
  - `ChatMessages.tsx`：空消息流保持空白，Relevant Notes / Suggested Prompts 这类辅助块不固定展示；Wridian 右侧消息流和输入组合入口为 `src/chat/ChatPanel.tsx`。
  - `ChatSingleMessage.tsx` / `ChatButtons.tsx`：用户消息使用浅边框背景，AI 消息不做重卡片；消息动作放在底部紧凑行。
  - pill 节点：Wridian 已按 Copilot 的 `BasePillNode` / `URLPillNode` / `ToolPillNode` / `PastePlugin` / `GenericPillSyncPlugin` 形态引入本地 `PromptPillNode`，真实注册到 Lexical 编辑树；URL、工具、文件、图片、记忆等上下文会从 Lexical 树同步回 prompt pill 状态。
  - 输入控制：Wridian 底部控制条只显示当前模型或当前项目名，不再提供 Project / Relevant / Vault 这类泛化工具按钮；文件 pill 会优先读取并缓存文件内容再注入上下文；粘贴 URL、保留的工具标记和图片会生成结构化 pill。
  - Project Mode / Relevant Notes：Wridian 的 Project Mode 已对齐作品项目，右侧下拉只提供“普通聊天”和作品项目文件夹名，不再提供手动“新建 Project”；打开作品文件时自动切换到所属作品项目。选择作品项目后，对话请求会读取创作记忆树中该项目的 `compressed.md` 压缩记忆。Relevant Notes 使用工作区本地全文词项重合 + wikilink/backlink 加权召回，点击可把相关稿件作为 file pill 注入。
- 记忆命中、注入和上下文选择默认在后台执行，不在右侧对话区常驻展示“本次使用的记忆”等系统说明；创作记忆树只由顶部“创作记忆树”动作打开。
- 工作界面右上角在创作记忆树图标右侧提供“知识图谱”入口；弹窗尺寸与创作记忆树一致，当前根据当前知识库 Markdown 分类、知识卡、正文 wikilink 和 frontmatter 中含 wikilink 的关系字段生成动态图谱视图；图谱支持自动适配视图、重置视图、鼠标位置缩放、拖拽画布、拖拽节点、悬浮预览知识卡和点击知识卡节点打开文件编辑区，点击文件夹节点只保留图谱浏览。知识图谱图标右侧提供“技能管理”入口，用于管理知识库运维、作品拆解、知识卡提炼和大神蒸馏等技能入口；对话输入框输入 `/` 时只显示当前已启用的技能。选择“知识库运维”时会注入一个 tool pill，包含当前知识库根目录、`zhishiku-skill` 来源状态和最小运维协议；当前版本不自动运行外部脚本、不自动改动知识库文件。
- 右侧侧边面板应保持“对话”语义，入口文案统一为“对话”。
- 对话回复可由模型返回结构化 `memories`，Wridian 自动写入创作记忆树 leaves；顶部“记忆树”按钮打开结构化 Markdown 记忆树，用户可查看、编辑和删除普通叶子文件。
- 当前已完成最小对话/记忆分离：底部输入调用对话命令，不再创建候选记忆或打开候选确认流；长期记忆写入后只通过记忆树抽屉管理。
- 聊天归档写入 `.wridian/chat/*.md` 时，用户输入、模型回复、选区和上下文 pill 内容必须作为 fenced text block 写入，避免污染 Markdown/frontmatter 结构。
- 正文 inline diff 的确认链路参考 `obsidian-copilot` 的 `replaceGuard.ts`，已接入替换保护：选区触发的修改优先验证选区 start/end/text 快照；无范围快照时只有 target 在当前正文中唯一命中且不与其他修改范围重叠才渲染和确认；找不到、重复出现或重叠的建议会保持待确认状态并提示需要重新定位，禁止默认改第一处。

## 作品类型

- 小说模式：章节、场景、人物、世界观、剧情线、伏笔、禁区、风格。
- 短剧/剧本模式：集、场、对白、转折、冲突、钩子、角色口吻、场地/预算限制、分集节奏。
- 当前前端会把 `.fountain` 文件，或包含多处内景/外景/集/场标记、角色对白行的文本识别为短剧/剧本模式；后端对话 prompt 会收到稿件类型并调整关注点。
- `.fountain` 不是普通可打开文件类型，后续应升级为剧本工作流：场景识别、角色对白、outline、预览和导出。
- UI 文案避免过早写死为“章节”；默认可用“稿件”“当前文件”“作品文件”，需要时按文件类型显示“章节 / 场景 / 剧本段落”。

## 借鉴边界

- `obsidian-copilot`：只借交互模型。正文稳定，AI 在侧栏/命令中辅助；参考 Vault QA、Relevant Notes、选中文本命令、Project Mode 和上下文选择。
- `claude-obsidian`：借 Markdown 知识图谱和 ingest 方法。通用知识库中的资料作为 sources，实体/概念拆分到知识图谱；作品项目只显式引用或采纳知识卡，不把通用知识自动写入创作记忆树。
- `OpenHuman`：只借 Memory Tree、Markdown vault、本地优先桌面结构；不借托管登录、OAuth、复杂集成、搜索代理和通用个人 AI OS 方向。
- `holaOS`：借 continuity 分层。当前写作现场、长期写作记忆、工作区/作品规则必须分开。
- `SillyTavern`：借 World Info、角色卡和插入规则，用在人物、设定、伏笔、禁区和风格条目上；不借角色聊天/RP 产品形态。
- `Beat`、`Better Fountain`、`Fountain`：借剧本纯文本格式、场景识别、outline、极简剧本编辑体验、预览和导出思路。
- `Twine`、`ink`、`Yarn Spinner`：后续借分支剧情、故事状态、节点式对白和条件逻辑，不作为 MVP。
- `Basic Memory`、`Graphiti`、`Pi`：后续分别参考 Markdown 语义图谱、时序事实/冲突检测、session tree/continue/fork；不进入最小 MVP。

## 记忆存储

- 记忆文件夹：Wridian 数据目录下的 `.wridian/memory-tree/`。
- 创作记忆树是用户可见、可编辑的 Markdown 生命树，主结构为根文件、九个分支文件和 leaves 叶子目录。
- 根文件：`.wridian/memory-tree/SOUL.md`、`AGENTS.md`、`MEMORY.md`。
  - `SOUL.md` 是图腾，记录 Wridian 的底层灵魂、价值观和对话人格。
  - `AGENTS.md` 是树根，记录 Wridian 如何行动、如何使用记忆树、哪些事必须问用户、哪些事不能自作主张。
  - `MEMORY.md` 是主干，记录索引、上下文编译策略、分支说明和最近活跃叶子。
- 分支文件：`.wridian/memory-tree/branches/` 下固定有 `SENSE.md`、`USER.md`、`RELATIONSHIP.md`、`JOURNEY.md`、`DRAMA.md`、`NOVEL.md`、`KNOWLEDGE.md`、`SKILL.md`、`AWARENESS.md`。分支文件只写机制、准则和如何长叶子，不写具体事件；其中 `KNOWLEDGE.md` 的中文名是“知识调用”，只记录创作记忆树如何调用外部知识库和知识图谱。
- 叶子目录：`.wridian/memory-tree/leaves/` 下按 `sense/user/relationship/journey/drama/novel/knowledge/skill/awareness` 分类。叶子文件才写具体生命记录、作品记忆、知识卡、技能和反思。
- 作品项目会自动在 `leaves/drama/` 或 `leaves/novel/` 下生成对应项目叶子文件和 `compressed.md` 压缩记忆文件；知识库 Markdown 不再复制成死副本，也不作为创作记忆树叶子展示，知识卡通过 `@` 显式选择和知识图谱入口读取。
- 记忆树画布中所有叶子文件显示为暖橙色 `#dc7d57` 小圆点，围绕对应分支主标签展示；悬浮提示文件名，点击后在画布左侧或右侧打开与主标签相同样式的内容编辑窗。
- 旧迁移主文件或 `legacy-*.md` 不能显示为叶子点；主标签文件内容未确定时，不用迁移文件伪造叶子。
- 记忆弹窗内部使用仿真树画布展示根、枝、叶；工作界面左侧作品库/知识库文件树不参与这套视觉变化。
- 对话完成后，模型可返回结构化 `memories`；Wridian 自动写入 `leaves/<branch>/` 普通 Markdown 叶子文件，用户在创作记忆树中直接编辑或删除。作品项目 `project.md` 和 `compressed.md` 属于项目核心记忆，只能编辑，不能通过叶子删除动作移除。
- 记忆作用域：普通聊天读取根文件和通用分支/叶子；作品项目额外读取命中的 drama/novel 分支机制和对应作品叶子；知识卡只在显式选择或召回时进入上下文，不默认混进作品记忆。
- 聊天记录：`.wridian/chat/*.md`，每个运行会话保存为 Markdown，包含 frontmatter、来源文件、用户/助手消息和上下文 pill。
- 旧的写入前预览和二次确认不再作为用户界面路径；记忆树里的 Markdown 文件是主编辑面。
- 对话上下文编译采用分层包：写作规则/作品设定、当前现场、记忆树命中、显式 pill 上下文、真实选区、当前稿件、用户输入分段渲染；pill 是文件/记忆引用和发送瞬间快照，不再伪装成选区文本。
- 知识图谱和相关稿件召回必须有本地扫描门禁：限制文件数、递归深度和单文件大小；可跳过的图谱问题返回 warnings 给前端展示，相关稿件读取错误继续显式失败。
- 后续上下文编译应参考 claude-obsidian、obsidian-copilot、OpenHuman、Hermes、OpenClaw、SillyTavern：按槽位、作用域、热上下文和预算加载记忆树文件，不把所有文件每轮硬塞进 prompt。

## 最小 MVP 路线

目标：先完成“本地稿件编辑 + 对话回答 + 显式记忆提取/确认 + 记忆注入”的闭环。

1. 修正对话/记忆交互混用。
   - 底部输入框发送后走对话流程，不再调用记忆写入，不再自动打开记忆树。
   - 增加“对话”侧边面板显示 AI 回复和可执行建议。
   - 保留“记忆”侧边面板，记忆提取只由显式按钮触发。
2. 实现最小对话请求。
   - 组装当前文件、选中文本或当前正文片段、active context、已确认相关记忆和用户输入。
   - 调用已配置的 OpenAI-compatible API，返回写作建议。
   - 第一版只展示回复，不自动改正文。
3. 做回复到正文的安全操作。
   - 支持用户选中正文片段并添加到输入框。
   - 对话回复底部支持重试、复制；用户消息底部支持编辑、复制。
   - Wridian 对正文的修改以正文内联 diff 展示，红色为删除、绿色为新增。
   - 文件顶部提供全部确认和全部取消；每处修改提供确认和取消。
   - 不做自动全文覆盖。
   - 当前已完成：正文区使用纯文本编辑器承载 inline diff，确认后写入正文并继续走已有自动保存链路；inline diff 只是编辑器内的待确认建议，不是单独审阅模式。
4. 完成记忆树注入闭环。
   - 对话上下文按记忆树槽位读取全局层、伙伴层和当前作品层文件。
   - 对话可自动沉淀模型提取的结构化长期记忆；用户通过“记忆树”抽屉直接编辑或删除普通叶子 Markdown 文件。
5. 补剧本 MVP。
   - 对 `.fountain` 或剧本稿件显示剧本上下文提示。
   - 对话输入框输入 `/` 可调用当前启用的创作技能，技能管理面板负责启用、停用和后续扩展。
6. 后续增强。
  - 实体/概念/来源拆分、Memory Tree 可视化、分支续接、时序冲突检测。

## 后端约束

- `src-tauri/src/lib.rs` 只负责模块声明、插件挂载和命令注册。
- 新业务逻辑必须进入对应模块；没有对应模块时先建小模块，不继续膨胀 `lib.rs`。
