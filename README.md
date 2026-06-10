# Wridian

Wridian 是一个面向小说作者、短剧编剧和设定型创作者的本地写作共创伙伴，核心卖点是“会长记忆的写作空间”。

它不是又一个通用笔记库，也不是把聊天窗口塞进编辑器的 AI 壳。Wridian 的目标更垂直：让你在同一个安静的桌面里写稿、对话、沉淀创作记忆、调用知识，并始终保持正文不被 AI 打断。

当前版本：`0.0.2`

## 一句话

一个更轻、更垂直、更容易上手的氛围写作工作台：左边是作品和知识，中间是稿件，右边是懂上下文的共创伙伴，背后是一棵持续生长的创作记忆树。

## 为什么做 Wridian

很多写作者不是缺一个更复杂的知识库，而是缺一个能陪自己持续进入创作状态的工具：

- 写到一半，需要一个伙伴帮你接住灵感，而不是跳到另一个聊天网页。
- 作品设定、人设、禁区、伏笔和口吻需要长期记住，而不是每次重新解释。
- 创作不是一次性问答，而是一个持续积累的过程；工具应该帮你记住作品如何生长。
- AI 可以提建议，但不能擅自改掉正文。
- 知识库应该服务创作现场，而不是把作者拖进复杂的配置和插件系统。

Wridian 因此把“稿件编辑 + AI 共创 + 创作记忆树 + 知识图谱”做成一个本地优先的写作空间。

## 和 Obsidian 有什么不同

Obsidian 很强，适合构建大型个人知识库和插件化工作流。Wridian 更克制，专注写作共创。

| 维度 | Wridian | Obsidian |
| --- | --- | --- |
| 定位 | 写作共创伙伴 | 通用知识库 |
| 上手 | 打开即是作品区、稿件区、对话区 | 需要自己搭插件和工作流 |
| AI 关系 | AI 常驻右侧，围绕当前稿件协作 | 多依赖插件或外部聊天 |
| 记忆 | 创作记忆树面向作品连续性、人物边界、禁区和进度 | 需要自行组织笔记体系 |
| 知识 | 知识卡服务创作现场，可显式注入上下文 | 更适合自由笔记和链接网络 |
| 写作安全 | AI 修改用正文 inline diff，必须确认 | 取决于插件和配置 |
| 氛围 | 少打扰、低配置、围绕写稿 | 高自由度、高可塑性 |

如果你已经有成熟的 Obsidian 系统，Wridian 不试图取代它。Wridian 更像一个专门用来写作、对话和沉浸创作的轻量桌面伙伴。

## 核心卖点

### 创作记忆树

Wridian 的记忆树不是普通文件夹，也不是聊天摘要列表。它面向作品连续性：人物边界、作品规则、当前进度、禁区、关系准则、项目压缩记忆和创作反思都能变成可编辑的 Markdown 叶子。

它的价值在于让 AI 不只是“这次回答得不错”，而是能随着作品一起记住：这个角色不能说什么、这条伏笔不能忘、这个项目现在写到哪里、这个世界观有哪些硬规则。

### 氛围写作

Wridian 的主界面保持三栏：作品文件、正文稿件、右侧对话。没有复杂仪表盘，没有泛化知识库首页，也不会用 AI 面板挤占正文。

### 写作共创伙伴

右侧对话区围绕当前稿件、选区、作品项目和显式引用的资料工作。你可以让它续写、拆解、复盘、改对白、分析人物动机，或把当前选区加入上下文继续讨论。

### 知识库和知识图谱

知识库用于存放人物、地点、设定、世界观、写作技法、资料摘录和拆解报告。知识图谱会根据 Markdown 分类、知识卡和 wikilink 生成本地图谱，支持拖拽、缩放、悬浮预览和点击打开。

### AI 不直接接管正文

AI 对正文的修改会以 inline diff 方式展示。你可以逐条确认或取消，也可以全部确认或全部取消。Wridian 默认不做全文覆盖。

### 技能开关

内置知识库运维、作品拆解、知识卡提炼、大神蒸馏等创作技能。开启后，在对话输入 `/` 时只显示当前启用的技能。

## 下载安装

Windows 安装包：

- [release/Wridian-0.0.2-x64-setup.exe](release/Wridian-0.0.2-x64-setup.exe)

也可以从 GitHub Release 下载：

- [Wridian 0.0.2](https://github.com/laoguo2025/wridian/releases/tag/v0.0.2)

下载后双击安装即可。当前安装包未做代码签名，Windows 可能会显示安全提醒。

## 本地优先

Wridian 以本地文件为主要事实来源：

- 作品库和知识库由用户选择本机目录。
- 聊天记录保存到 `.wridian/chat/`。
- 创作记忆树保存到 `.wridian/memory-tree/`。
- 删除文件时只移动到当前工作根目录的 `.wridian-trash/`，不做永久删除。
- 当前支持一个 OpenAI-compatible 自定义 API，模型账户由用户自己配置。

## 适合谁

- 小说作者：章节、场景、人物、伏笔、禁区、世界观和风格统一。
- 短剧编剧：分集、场景、对白、钩子、冲突和角色口吻打磨。
- 设定型创作者：资料、知识卡、人物原型、故事模型和写作技法沉淀。
- 喜欢 Obsidian 的本地文件理念，但不想为写作共创维护一整套插件系统的人。

## 开发

环境要求：

- Node.js
- Rust
- Windows 上需要 Visual Studio Build Tools C++ 工具链

常用命令：

```powershell
npm install
npm run dev
npm run build
powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml
```

打包 Windows 安装包：

```powershell
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
cmd.exe /d /s /c "`"$vcvars`" >nul && npm run tauri -- build"
```

构建产物位置：

- 主程序：`src-tauri/target/release/wridian.exe`
- NSIS 安装包：`src-tauri/target/release/bundle/nsis/`
- MSI 安装包：`src-tauri/target/release/bundle/msi/`

## 项目结构

- `src/App.tsx`：主应用状态和页面编排。
- `src/chat/`：右侧对话、消息仓库、上下文 pill、对话请求和持久化。
- `src/editor/`：正文编辑器、inline diff 和稿件类型识别。
- `src/files/`：左侧文件树和右键菜单。
- `src/memory/`：创作记忆树抽屉。
- `src/knowledge/`：知识图谱和知识卡建议索引。
- `src/settings/`：模型账户设置。
- `src/skills/`：技能管理抽屉。
- `src-tauri/src/`：Tauri 后端命令、工作区、记忆、知识图谱、模型账户和对话上下文。

## 当前边界

- 目前只支持一个 OpenAI-compatible 自定义 API。
- 暂不接入生图、生视频和复杂模型网关。
- 知识卡不会自动变成作品记忆，进入作品上下文需要显式引用、采纳或改写。
- AI 对正文的修改需要用户确认，不自动全文覆盖。
