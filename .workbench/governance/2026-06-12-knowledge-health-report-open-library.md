# 2026-06-12 Knowledge Health Report Open Library

## 背景

用户反馈在知识图谱体检结果中点击“打开报告”后，编辑区标题变成知识库体检报告，但正文仍显示作品库文件内容；底部提示“文件不在当前 Wridian 工作目录内”，左侧文件树也没有切到知识库标签。

## 原因

知识图谱报告打开链路复用通用 `openFilePath(path)`，没有携带目标库语义，也没有在报告刚生成后先刷新 workspace 文件树。编辑器还会在真实读取成功前先设置标题，导致失败时出现标题和正文错配。

## 改动

- `openFilePath` 支持 `targetLibrary` 和 `refreshBeforeOpen`。
- 从文件树打开文件时按节点 `library` 切换左侧标签。
- 从知识图谱打开知识卡或体检报告时强制切到知识库，并在打开前刷新 workspace。
- 刷新失败时停止打开，避免半切换状态。
- 移除 editable 文件读取成功前的预设标题，失败时保留原正文和标题一致。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_graph --lib` 通过，5 个测试。

## 回退

回退 `src/App.tsx` 本次修改即可恢复旧打开行为。
