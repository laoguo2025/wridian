# Wridian Project Map

## 定位

Wridian 是独立桌面写作共创系统，当前优先级是本地写作文件、写作记忆和简化模型接入。

## 当前入口

- 前端入口：`src/App.tsx`
- 主要样式：`src/App.css`
- Tauri 组装入口：`src-tauri/src/lib.rs`
- 后端模块：
  - `src-tauri/src/runtime.rs`：本地数据目录、默认 Vault、运行时文件路径。
  - `src-tauri/src/workspace.rs`：本地作品目录、文件树、正文读写。
  - `src-tauri/src/model_accounts.rs`：自定义 OpenAI-compatible API 配置和测试。
  - `src-tauri/src/memory.rs`：待确认记忆和写入记忆。
- 本地运行：`npm run dev`

## 当前边界

- 本地文件只支持 `md`、`markdown`、`txt`、`fountain`。
- 文件读写只允许默认 Vault 或用户选择的工作目录内文件。
- 文件区采用 Obsidian 式结构：顶部新建文件/文件夹/作品文件夹，树节点支持多层级展开/收回和右键菜单，底部系统设置。
- 文件区“移到回收站”只移动到当前工作根目录 `.wridian-trash/`，不做永久删除。
- 模型接入先支持一个 OpenAI-compatible 自定义 API。
- 记忆 MVP 使用 `.wridian/memory-tree.json` 和 `.wridian/candidates.json`，先做本地确认闭环，再接模型抽取。
- 暂不接入生图、生视频和复杂模型网关。

## 记忆存储

- 记忆文件夹：Wridian 数据目录下的 `.wridian/`。
- 长期记忆：`.wridian/memory-tree.json`。
- 候选记忆：`.wridian/candidates.json`。
- 当前分类只有“长期记忆”和“待确认候选”；后续应扩展为作品、人物、世界观、风格、禁区等写作分类。
- 模型提取不得直接写入长期记忆，必须经过候选、编辑、确认。

## 后端约束

- `src-tauri/src/lib.rs` 只负责模块声明、插件挂载和命令注册。
- 新业务逻辑必须进入对应模块；没有对应模块时先建小模块，不继续膨胀 `lib.rs`。
