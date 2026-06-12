# Wridian Project Map

## 定位

Wridian 是独立桌面写作对话系统，当前优先级是本地写作文件、写作记忆、对话和简化模型接入。

Wridian 不只用于写小说，也用于短剧剧本、剧本、分集大纲、场景稿、人物小传和设定资料。产品定位是“带写作记忆的本地写作对话系统”，不是通用 AI OS、知识库问答壳、模型供应商管理器或只服务长篇小说的编辑器。

第一屏应保持极简：作品文件区、稿件编辑区、右侧对话区。记忆、模型、人物、设定、历史和抽取动作进入侧边面板或设置，不抢占稿件编辑区。浅色主题应使用柔和纸灰层级，不使用大面积纯白盒子；分栏拖拽条默认弱化，仅 hover/拖动时显出 Wridian 暖橙。

## 当前入口

- 前端入口：`src/App.tsx`
- 右侧对话区：`src/chat/ChatPanel.tsx`
- 正文编辑器：`src/editor/DraftEditor.tsx`
- 聊天运行管理：`src/chat/chatManager.ts`
- 聊天持久化：前端 `src/chat/chatPersistence.ts`，后端 `src-tauri/src/chat_persistence.rs`
- 聊天输入组件：`src/chat/WridianPromptEditor.tsx`
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
  - `src-tauri/src/model_accounts.rs`：多供应商模型账户配置、凭据存储、模型切换和连接测试。
- NSIS 卸载清数据 hook：`src-tauri/nsis-hooks.nsh`，用于补齐 Tauri 默认清理路径未覆盖的 `$APPDATA\Wridian` / `$LOCALAPPDATA\Wridian`、当前配置知识库及常见用户目录下 `Wridian知识库` 的 Wridian 运行产物（`.wridian`、`.wridian-trash`、`hot.md`、fold、体检报告）和 Wridian 已知模型供应商 Windows 凭据目标；不会直接删除整个知识库根目录。
- `src-tauri/src/memory.rs`：文件化记忆树、作用域记忆和知识卡读取。
  - `src-tauri/src/cocreation.rs`：对话请求上下文组装和模型回复。
- `src-tauri/src/bridge.rs`：作品域和知识域之间的显式 frontmatter 关系写入命令。
- 本地运行：`npm run dev`
- Rust 检查：普通 PowerShell 中优先运行 `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`；脚本会定位本机 Visual Studio Build Tools 并进入 MSVC 环境，仓库不再提交 `.cargo/config.toml` 这类本机绝对路径配置。

## 当前边界

- 文件树可显示常见资料文件：`md`、`markdown`、`txt`、`doc/docx/wps`、`pdf`、`png/jpg/jpeg/webp/gif/svg/bmp`、`csv/xls/xlsx/et`、`ppt/pptx/dps`、`json/yaml/yml`。中间文件区统一承载查看体验：`md/markdown/txt/docx` 可直接编辑并自动保存，`pdf` 和图片直接预览，`csv/json/yaml/yml` 只读文本查看；旧二进制 Word/WPS（`doc/wps`）必须通过明确转换引擎后才能在 Wridian 内安全编辑，不允许按 UTF-8 或二进制原样写坏文件。
- 直接打开/点名读取的文本类文件有大小门禁，当前上限为 512KB；PDF 和图片预览走 base64 IPC 前有 20MB 上限。元数据索引、相关笔记、知识检索和直接预览必须保持“先看大小再读内容”的一致门禁。
- `docx` 当前只允许保存纯文本结构；若正文 XML 包含表格、脚注、尾注、批注、修订、绘图、超链接或内容控件等复杂语义，保存必须拒绝并提示，避免静默破坏原文档。复杂 Word/WPS 的完整保真编辑仍需明确转换/编辑引擎后再接入。
- 作品库面向用户时必须先由用户选择本地文件夹；知识库安装后默认在本机 D 盘创建并启用 `Wridian知识库` 根目录，若无 D 盘则回落到 Wridian 本地数据目录。用户仍可手动改选其他知识库根目录。
- 前端文件树节点同时携带绝对路径和相对路径：绝对路径只用于 Tauri 本地安全校验和读写，相对路径用于 UI、聊天上下文引用和跨库引用语义。
- 对话请求默认携带作品库和知识库的相对文件树，不向模型暴露本机绝对路径。模型若要增、改、删文件树，必须返回 `fileOperations`，后端只按当前作品库/知识库内的相对路径执行 `writeFile/createFolder/rename/trash`，并继续复用本地安全校验；前端收到成功操作后刷新文件树。
- 模型返回的 `fileOperations` 每次执行都必须在 Wridian 运行目录写入 `.wridian/runtime/model-file-operations.jsonl` 审计记录，至少包含时间、action、library、path、newName、执行结果和旧目标的存在性/类型/大小/sha256 摘要；审计不复制完整文稿内容，用作回退依据和问题追溯。
- 文件区采用 Obsidian 式结构：顶部新建文件/文件夹/作品文件夹，树节点支持多层级展开/收回和右键菜单，底部系统设置。
- 左侧文件区分为“作品库 / 知识库”标签页：作品库放作品项目、章节、剧本、分集、场景稿；知识库放人物、地点、设定、世界观、风格、禁区和资料摘录等知识卡。默认知识库分类模板为 `00知识库治理`、`01原始资料`、`02拆解报告`、`03故事模型`、`04人物原型`、`05情节方程`、`06写作技法`、`07综合素材`、`08大神蒸馏`、`09文件归档`，其中 `00知识库治理` 默认包含使用说明；这些只是初始分类，用户可以在文件树里增、改、删，原生知识库体检会按实际目录给出修复建议。
- 左侧文件区底部按当前标签页展示已绑定根目录，格式为“当前目录：目录名”，完整路径通过悬停标题查看；该提示只读，不改变作品库/知识库选择逻辑。
- 文件区“移到回收站”调用用户本机系统回收站，不再移动到库内 `.wridian-trash/`；路径仍必须先通过当前作品库或知识库边界校验。
- 模型接入支持多供应商账户和多模型切换：前端唯一供应商目录入口为 `src/settings/providerCatalog.ts`，保存 `presetKey/providerType/protocol/authStyle/baseUrl/defaultModels/extraEnv`。当前协议名为 `anthropic`、`openai-compatible`、`google`；不再使用自研 `openai/gemini` 协议枚举。用户界面不暴露参考源码项目名。第三方 API 只保留通用 `Anthropic Third-party API` 和 `OpenAI-Compatible API` 入口，不为具体厂商重复展示独立直连卡片；Anthropic 兼容链路会在非流式无文本时自动用流式重试，并解析常见 JSON/SSE 兼容返回。
- 官方模型别名和 Google Code Assist 请求头有环境变量覆盖口径：`WRIDIAN_ANTHROPIC_HAIKU_MODEL`、`WRIDIAN_ANTHROPIC_SONNET_MODEL`、`WRIDIAN_ANTHROPIC_OPUS_MODEL`、`WRIDIAN_GOOGLE_CODE_ASSIST_USER_AGENT`、`WRIDIAN_GOOGLE_CODE_ASSIST_API_CLIENT`。Gemini OAuth client 仍支持 `WRIDIAN_GOOGLE_OAUTH_CLIENT_ID` 和 `WRIDIAN_GOOGLE_OAUTH_CLIENT_SECRET` 高级覆盖；默认值只作为公共桌面客户端兼容兜底。
- 模型设置弹窗入口为 `src/settings/ModelSettingsDialog.tsx`，尺寸与知识图谱一致，采用上下布局：“已连接服务”在上，“添加服务”在下。添加服务按“授权登录 / 国内服务 / 第三方API”分组，同页三列展示；Aliyun Bailian Coding Plan 与 Aliyun Bailian Token Plan 是两个独立 provider；所有 provider 卡片为“名称/描述 + 右侧连接或断开按钮”，不显示头像图标、底部接入类型标签或已连接详情表；已配置 provider 从添加区消失，取消配置后恢复。
- 授权登录支持 Anthropic、OpenAI、Gemini：Anthropic 采用 Claude PKCE code flow；OpenAI 采用 ChatGPT/Codex device-code flow，先向 `auth.openai.com/api/accounts/deviceauth/usercode` 获取验证码，再到 `auth.openai.com/codex/device` 登录并换取 Codex token，运行时走 `https://chatgpt.com/backend-api/codex/responses`；Gemini 采用 Google Gemini CLI / Code Assist OAuth，监听 `127.0.0.1:8085/oauth2callback`（占用时回落临时端口），默认使用 Gemini CLI 同款公共桌面 OAuth client，本机环境变量 `WRIDIAN_GOOGLE_OAUTH_CLIENT_ID` 和 `WRIDIAN_GOOGLE_OAUTH_CLIENT_SECRET` 仅作为高级覆盖项，不在模型账号弹窗展示。三者 OAuth JSON 均写入 Windows Credential Manager，调用时读取 access token，过期前用 refresh token 刷新。
- 模型请求 endpoint 统一由后端构造：Anthropic 兼容供应商把 catalog Base URL 作为 SDK base URL，实际请求 `/v1/messages`，并识别用户填写的完整 `/v1/messages` endpoint；OpenAI-compatible 识别完整 `/chat/completions` endpoint，否则补 `/v1/chat/completions`，特殊请求体参数只由 `extraEnv` 显式配置（`WRIDIAN_OPENAI_COMPAT_MAX_TOKENS_FIELD`、`WRIDIAN_OPENAI_COMPAT_OMIT_TEMPERATURE`、`WRIDIAN_OPENAI_COMPAT_THINKING`）；OpenAI OAuth 仍使用 `/responses`；Gemini API Key 版使用 `https://generativelanguage.googleapis.com/v1beta/models/<model>:generateContent`，Gemini OAuth 版使用 `cloudcode-pa://google` 作为内部 marker，实际请求 `https://cloudcode-pa.googleapis.com/v1internal:generateContent`，并按 Code Assist 要求包装 `project/model/user_prompt_id/request`。
- 模型账户配置运行时存放在 Wridian 数据目录的 `.wridian/model-accounts.json`；供应商、Base URL、模型名、`authStyle`、`extraEnv` 和当前选中模型写入本地配置文件，API Key/访问凭据写入 Windows Credential Manager，目标名按 `provider:<provider-id>.ai.wridian.app` 组织。旧 `customApi` 配置会迁移到多供应商结构；安装包不写入用户密钥。
- 记忆系统以 `.wridian/memory-tree/` 下的 Markdown 文件树为主入口；用户在“创作记忆树”抽屉直接查看和编辑全局层、伙伴层、作品层和知识调用机制文件。
- 知识、知识库、知识卡和知识图谱属于作品项目之外的通用知识积累；作品项目可显式引用知识卡，但知识卡不自动变成作品记忆。
- 暂不接入生图、生视频和复杂模型网关。

## 作品域与知识域

- Frontmatter 关系协议唯一 owner：`.workbench/doc/wridian-frontmatter-relation-protocol.md`。
- 作品域负责正在写的东西：作品项目、作品库、稿件、相关元素、创作记忆树、Project Mode、Relevant Notes、选区命令和上下文选择。
- 创作记忆树是作品项目的记忆与规则树；它保存作品连续性、项目规则、人物边界、禁区、当前进度和项目压缩记忆。
- 作品相关元素可以引用知识卡，但不拥有知识卡；引用知识卡不等于把知识卡写成作品记忆。
- 知识域负责作品项目之外的通用知识积累：知识、知识库、知识卡、知识图谱、原生知识库体检产物、sources/entities/concepts 口径、交叉引用和 hot cache。
- 知识图谱是作品项目之外的通用知识网络；它服务多个作品，不归属于单个作品项目。
- 知识卡可以被多个作品引用，但不会自动变成任何作品的记忆。
- 从知识到作品只能通过“引用 / 采纳 / 改写成作品设定”进入项目。
- 从作品到知识只能通过“摘录 / 抽象 / 沉淀为知识卡”离开项目。
- `obsidian-copilot` 只借给作品域：Project Mode、Relevant Notes、选区命令、上下文选择。
- `claude-obsidian` 只借给知识域：图谱索引、sources/entities/concepts 口径、交叉引用、hot cache；知识生产流程由作品拆解、知识卡提炼和大神蒸馏三个内置技能承担；知识库体检是 Wridian 原生工作流，不再作为 skill。
- `tolaria` 借给作品域和知识域之间的边界：frontmatter 关系、capture/organize、文件系统唯一真相。
- 知识库不使用“未结构化文件候选箱”判定知识卡是否正式。`01原始资料` 本来就是未加工素材，`02拆解报告` 保存分析产物和 A/B/C 候选，只有通过知识卡提炼质量闸门的 S 级卡才能写入 `03-07` 成为正式知识卡。

## 交互边界

- 稿件编辑区只负责当前文件内容，不承载聊天历史，不因对话回复而挤占正文。
- 软件启动后不默认展示示例作品；未选择文件时稿件编辑区为空背景，只在中间显示 Wridian 字标、slogan“让故事有记忆，让知识可调用”，以及“选择作品库 / 查看记忆树”两个轻入口。
- 稿件编辑区始终以纯文本写作体验为主，不做 Markdown 预览或独立审阅模式；小说作者和短剧编剧默认不依赖 Markdown 格式效果。`docx` 进入纯文本编辑和自动保存链路，当前按 WordprocessingML 正文抽取/回写，不承诺保留复杂 Word 样式；`doc/wps` 后续接入 MarkItDown/LibreOffice/PaddleOCR 等转换或 OCR 引擎前只展示转换需求，不做危险写入。
- 正文长度只允许稿件编辑区内部滚动，不允许撑出整个工作界面的窗口级上下滚动条。
- 底部对话输入应进入对话流程，不得直接创建记忆候选或自动打开记忆抽屉。
- 对话区常驻在工作界面右侧，按 `obsidian-copilot` 的简洁侧栏聊天形态复刻可见交互：消息流为空时不显示说明卡片，输入框位于对话区底部，发送后只更新右侧消息流，不弹出额外抽屉。
- 当前对齐的 `obsidian-copilot` 源码基线：
  - `ChatInput.tsx`：带边框的底部输入容器、上下文 pill 区、中间约 60px 起步输入区、24px 底部工具栏、小发送/停止动作；发送后按钮切换为“停止”，停止会取消当前后端请求、收口 pending 状态，并禁止本轮助手消息、正文修改和长期记忆写入继续落地。
  - `LexicalEditor.tsx`：输入区内部滚动，长文本不撑高右栏；Wridian 聊天输入区已从 textarea 切换为 Lexical `ContentEditable`，使用受控文本同步、历史插件和 Enter 发送；实现入口为 `src/chat/WridianPromptEditor.tsx`。
  - `AtMentionCommandPlugin.tsx` / `SlashCommandPlugin.tsx`：Wridian 已接入本地第一版 `@` 知识卡选择和 `/` 技能调用提示，实现在 `src/chat/WridianPromptEditor.tsx` 内。`@` 菜单只选择知识库内容，先显示知识库下含 Markdown 知识卡的分类文件夹，选中分类后再显示该分类下的知识卡；选中知识卡后读取文件内容并以 memory pill 注入上下文。`/` 菜单只显示“技能管理”中当前启用的技能；选择任一技能都会注入普通 tool pill，并随发送进入对话请求。
    - 剧本模式：前端按 `.fountain` 扩展名、内景/外景/集/场信号和角色对白行识别短剧/剧本稿件；稿件类型会进入对话请求，`/` 菜单仍只显示“技能管理”中当前启用的技能。
  - 文件/上下文检索：右键文件可把文件内容作为 `file` pill 注入；用户在对话中明确谈到作品库或知识库文件树里的文件名/相对路径时，后端可在受限范围内读取该文件作为“点名文件”上下文。右侧对话区的 Relevant Notes 只在作品库文件打开时检索作品域候选，不自动混入知识库知识卡；知识卡通过 `@` 显式选择或桥接关系进入作品上下文。`@` 菜单不再搜索作品文件，只搜索知识卡。
  - `ContextManager.ts` / `PromptContextTypes.ts`：Wridian 已开始拆出聊天上下文边界，`src/chat/promptContext.ts` 负责 prompt pill 类型、序列化、上下文建议构造和写作命令建议；消息仓库只保存消息和已绑定的上下文快照。
  - `MessageRepository.ts`：Wridian 已开始拆出前端消息仓库边界，`src/chat/messageRepository.ts` 负责消息类型、ID、用户/助手消息创建、编辑恢复和重试定位；`App.tsx` 仍负责调用 Tauri 对话命令。
  - `ChatManager.ts` / `ChatPersistenceManager.ts`：Wridian 已引入本地前端版 `src/chat/chatManager.ts`，负责消息列表、pending/error、发送对话请求、追加助手回复和生成待确认正文修改；聊天记录通过 `src/chat/chatPersistence.ts` 调用后端 `src-tauri/src/chat_persistence.rs` 保存为 `.wridian/chat/<session>.md`。
  - `ChatMessages.tsx`：空消息流保持空白，Relevant Notes / Suggested Prompts 这类辅助块不固定展示；Wridian 右侧消息流和输入组合入口为 `src/chat/ChatPanel.tsx`。
  - `ChatSingleMessage.tsx` / `ChatButtons.tsx`：用户消息使用浅边框背景，AI 消息不做重卡片；消息动作放在底部紧凑行。
  - pill 节点：Wridian 已按 Copilot 的 `BasePillNode` / `URLPillNode` / `ToolPillNode` / `PastePlugin` / `GenericPillSyncPlugin` 形态引入本地 `PromptPillNode`，真实注册到 Lexical 编辑树；URL、工具、文件、图片、记忆等上下文会从 Lexical 树同步回 prompt pill 状态。
  - 输入控制：Wridian 底部控制条只显示当前模型或当前项目名，不再提供 Project / Relevant / Vault 这类泛化工具按钮；文件 pill 会优先读取并缓存文件内容再注入上下文；粘贴 URL、保留的工具标记和图片会生成结构化 pill。
  - Project Mode / 点名文件：Wridian 的 Project Mode 已对齐作品项目，右侧下拉只提供“普通聊天”和作品项目文件夹名，不再提供手动“新建 Project”；打开作品文件时自动切换到所属作品项目。选择作品项目后，对话请求会读取创作记忆树中该项目的续接记忆包：`project.md`、`compressed.md` 和少量同项目必要叶子，不会把全知识库塞进作品上下文。文件树会以相对路径进入对话请求，用户明确谈到作品库或知识库中的文件时，Wridian 可读取该文件内容；需要新建、修改文件/文件名或删除文件时，模型必须返回受限 `fileOperations`，删除实际为移到用户本机系统回收站。
- 记忆命中、注入和上下文选择默认在后台执行，不在右侧对话区常驻展示“本次使用的记忆”等系统说明；创作记忆树只由顶部“创作记忆树”动作打开。
- 工作界面右上角在创作记忆树图标右侧提供“知识图谱”入口；弹窗尺寸与创作记忆树一致，当前根据当前知识库 Markdown 分类、知识卡、wikilink 和 frontmatter 中包含 `[[wikilink]]` 的关系字段生成动态图谱视图；图谱会按 frontmatter `type/kind/card_type/wridian_type` 或默认 00-09 目录推断节点类型，并用关系字段名显示 typed relation。图谱基于 Metadata Index 暴露 aliases、tags、source refs、出链、反链、断链和被引用来源；Wridian 生成的 `hot.md`、fold 和体检报告不作为知识卡节点、检索候选或健康分母；未解析 wikilink 会作为断链节点显示，但不能点击打开文件。图谱视觉上必须区分分类节点、知识卡节点、断链节点、强关系边和引用边，避免退化成无语义散点图。图谱前端采用 d3-force 布局加单 Canvas 渲染，保留节点/边规模上限、自动适配视图、重置视图、鼠标位置缩放、拖拽画布、拖拽节点、悬浮显示库内相对路径和元数据但不显示文件正文，点击知识卡节点打开文件编辑区，点击文件夹节点只保留图谱浏览。知识图谱弹窗右上角提供“知识库体检”原生入口，自动刷新 manifest、更新 `hot.md`、生成 fold、运行结构/关系/来源/孤岛/skill 化候选扫描，并写入本次时间戳命名的 `00知识库治理/知识库体检-YYYYMMDDTHHMMSS*.md`；体检结果面板提供打开报告和一键修复，一键修复只执行低风险确定性动作，高风险语义治理写入待确认清单。知识图谱图标右侧提供“技能管理”入口，仅管理作品拆解、知识卡提炼和大神蒸馏三个创作技能；三个技能作为安装包内置资源随包分发，资源根分别为 `resources/skills/work-decompose/`、`resources/skills/knowledge-card/`、`resources/skills/author-distill/`；对话输入框输入 `/` 时只显示当前已启用的三个创作技能。
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
- 作品项目会自动在 `leaves/drama/` 或 `leaves/novel/` 下生成对应项目记忆分组，其中 `project.md` 是项目长期记忆，`compressed.md` 是项目压缩记忆；二者都只作为内部项目续接上下文读取，不作为用户可见叶子点展示。其他同项目 Markdown 是必要续接叶子；知识库 Markdown 不再复制成死副本，也不作为创作记忆树叶子展示，知识卡通过 `@` 显式选择和知识图谱入口读取。
- 记忆树画布中用户可见叶子文件显示为暖橙色 `#dc7d57` 小圆点，围绕对应分支主标签展示；内部运行文件、fold、体检报告、压缩文件和知识库缓存文件不显示为叶子点。
- 创作记忆树抽屉支持按作品项目过滤；过滤后只显示该项目的作品连续性记忆和核心续接文件，不显示通用知识库内容。
- 旧迁移主文件或 `legacy-*.md` 不能显示为叶子点；主标签文件内容未确定时，不用迁移文件伪造叶子。
- 记忆弹窗内部使用仿真树画布展示根、枝、叶；工作界面左侧作品库/知识库文件树不参与这套视觉变化。
- 对话完成后，模型可返回结构化 `memories`；Wridian 自动写入 `leaves/<branch>/` 普通 Markdown 叶子文件，用户在创作记忆树中直接编辑或删除。作品项目 `project.md` 和 `compressed.md` 属于内部项目记忆，不作为记忆树叶子展示，也不能通过叶子删除动作移除。
- 记忆作用域：普通聊天读取根文件和通用分支/叶子；作品项目额外读取命中的 drama/novel 分支机制、对应作品核心记忆和必要续接叶子；知识卡只在显式选择或召回时进入上下文，不默认混进作品记忆。
- 聊天记录：`.wridian/chat/*.md`，每个运行会话保存为 Markdown，包含 frontmatter、来源文件、用户/助手消息和发送瞬间冻结的上下文 pill。
- 对话续接：`.wridian/active-context.json` 保存当前作品、当前片段、上次用户意图、上次判断、下一步建议和 compact summary；`.wridian/chat/session-index.json` 按“普通聊天”和作品项目分别记录 active session；`.wridian/chat/sessions/<session>.json` 保存可恢复消息树和消息时间戳；`.wridian/chat/session-history/<session>.jsonl` 追加每轮快照；`.wridian/chat/compact-summary.md` 保存创作交接卡。右侧对话区按消息时间线显性保留历史，对话切换项目时必须切换到该项目自己的对话页。
- 上下文注入 UI 只展示本轮实际命中的槽位，不展示“命中 n/总数”或未命中槽位；槽位文案面向用户使用中文：当前稿件、规则路由、项目记忆、最近对话现场、压缩记忆、已选知识卡、点名文件、技能规则、本次请求。
- 对话消息操作：用户消息气泡外侧底部右下角只显示图标按钮“上下文 / 修改 / 复制”，但只有当前对话最新一轮用户消息允许显示“修改”；历史用户消息只保留上下文和复制。修改会按豆包式“取消 / 整行圆角文本编辑框 / 提交”形态直接把原消息行替换为编辑条，不再保留外层消息气泡或短气泡尺寸；提交按钮使用 Wridian 暖橙主题色且不带光晕，编辑框不得越出右侧对话区。助手消息气泡外侧底部右下角只显示“重试 / 复制”，不显示“上下文”或“分叉”。上下文命中列表显示在上下文图标底部，复制成功提示显示在复制图标底部。历史分叉字段仍可兼容读取，但不作为当前可见操作入口。
- 旧的写入前预览和二次确认不再作为用户界面路径；记忆树里的 Markdown 文件是主编辑面。
- 对话上下文编译采用固定槽位和预算：当前稿件/选区、作品库和知识库文件树、规则路由、项目记忆、最近对话现场、压缩记忆、已选知识卡、点名文件、技能规则、用户请求分段渲染；规则路由只读取当前作品库和知识库根目录的 `WRIDIAN.md`、`AGENT.md`、`AGENTS.md`、`index.md`、`hot.md`，用于说明工作规则、导航和近期上下文，不写入创作记忆树或技能规则槽位；tool pill 单独进入技能规则槽位，pill 是文件/记忆/技能引用和发送瞬间快照，不再伪装成选区文本。
- 知识图谱和文件树读取必须有本地扫描门禁：限制文件数、递归深度和单文件大小；可跳过的图谱问题返回 warnings 给前端展示，点名文件读取错误不应阻断普通对话。
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
   - 对话回复底部支持重试、复制；用户消息底部支持修改、复制。
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
  - 实体/概念/来源拆分、时序冲突检测。

## 后端约束

- `src-tauri/src/lib.rs` 只负责模块声明、插件挂载和命令注册。
- 新业务逻辑必须进入对应模块；没有对应模块时先建小模块，不继续膨胀 `lib.rs`。
