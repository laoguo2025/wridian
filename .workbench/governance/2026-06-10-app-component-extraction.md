# App 组件拆分留痕

## 目标

降低 `src/App.tsx` 维护瓶颈，先拆出用户点名的四个高耦合 UI 区域：正文编辑器、创作记忆树抽屉、知识图谱抽屉和模型设置对话框。

## 变更

- `src/editor/DraftEditor.tsx` 承载正文 contenteditable、inline diff 展示、选区读取和光标恢复。
- `src/memory/MemoryDrawer.tsx` 承载创作记忆树抽屉、树视图模型和叶子编辑/删除 UI。
- `src/knowledge/KnowledgeGraphDrawer.tsx` 承载知识图谱抽屉、Canvas 布局、预览读取和交互相机。
- `src/settings/ModelSettingsDialog.tsx` 承载模型账户读取、保存和测试 UI。
- `src/appTypes.ts` 汇总前端与 Tauri 命令之间共享的数据形状。
- `src/numberUtils.ts` 提供原 `App.tsx` 中已有的 `clamp` 行为，保留 `max < min` 时回落到 `min` 的语义。

## 非变化约束

- 不改 CSS 类名、中文文案、Tauri 命令名、状态流转和用户交互入口。
- 技能管理抽屉仍暂留 `App.tsx`，避免本轮扩大到用户未点名的功能面。
- `App.tsx` 仍是应用组装入口，负责全局状态、文件操作和跨组件编排。

## 回退

本轮是纯前端模块搬迁；如出现回归，可回退本次提交，恢复 `App.tsx` 内联组件定义。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，26 个 Rust 测试全部通过。
