# Wridian Project Map

## 定位

Wridian 是独立桌面写作共创系统，当前优先级是本地写作文件、写作记忆和简化模型接入。

## 当前入口

- 前端入口：`src/App.tsx`
- 主要样式：`src/App.css`
- Tauri 命令：`src-tauri/src/lib.rs`
- 本地运行：`npm run dev`

## 当前边界

- 本地文件只支持 `md`、`markdown`、`txt`、`fountain`。
- 文件读写只允许默认 Vault 或用户选择的工作目录内文件。
- 模型接入先支持一个 OpenAI-compatible 自定义 API。
- 暂不接入生图、生视频和复杂模型网关。

