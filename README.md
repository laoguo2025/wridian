# Wridian

Wridian 是一个本地优先的写作对话桌面应用。它面向小说、短剧剧本、分集大纲、场景稿、人物小传和设定资料等创作场景，把本地稿件编辑、右侧 AI 共创对话、创作记忆树和知识库图谱放在同一个工作台里。

当前版本：`0.0.1`

## 下载安装

Windows 安装包在仓库内：

- [release/Wridian-0.0.1-x64-setup.exe](release/Wridian-0.0.1-x64-setup.exe)

下载后双击安装即可。当前安装包未做代码签名，Windows 可能会显示安全提醒。

## 核心功能

- 本地作品库：选择本机文件夹作为作品库，支持 `md`、`markdown`、`txt`、`fountain` 稿件。
- 知识库：独立管理人物、地点、设定、世界观、风格、资料摘录和写作技法等知识卡。
- 右侧共创对话：稿件编辑区保持稳定，AI 对话常驻右侧，不挤占正文。
- 上下文引用：可把当前文件、选区、相关稿件和知识卡加入对话上下文。
- 创作记忆树：模型回复可沉淀结构化长期记忆，用户可在记忆树抽屉中查看、编辑和删除普通叶子文件。
- 知识图谱：根据知识库 Markdown 分类、知识卡和 wikilink 生成本地图谱，支持拖拽、缩放、悬浮预览和点击打开知识卡。
- 正文安全修改：AI 建议以正文内联 diff 展示，用户逐条确认或取消，不默认覆盖全文。
- 技能管理：内置知识库运维、作品拆解、知识卡提炼和大神蒸馏等技能开关；对话输入 `/` 时只显示已启用技能。
- 模型账户：支持一个 OpenAI-compatible 自定义 API，配置测试会验证连接和响应格式。

## 本地数据

Wridian 以本地文件为主要事实来源：

- 作品库和知识库由用户选择本机目录。
- 聊天记录保存到 `.wridian/chat/`。
- 创作记忆树保存到 `.wridian/memory-tree/`。
- 删除文件时只移动到当前工作根目录的 `.wridian-trash/`，不做永久删除。

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
