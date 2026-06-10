# App 二次瘦身留痕

## 目标

继续降低 `src/App.tsx` 维护压力，把首轮未拆出的纯 UI 和纯 helper 从主应用编排中移出。

## 变更

- `src/files/FileTree.tsx` 承载左侧文件树递归渲染和文件右键菜单。
- `src/skills/CreativeSkillsDrawer.tsx` 承载技能管理抽屉。
- `src/knowledge/knowledgeSuggestions.ts` 承载知识库 Markdown 分类和知识卡建议索引。
- `src/icons.tsx` 承载原 `App.tsx` 内联 SVG 图标组件。

## 非变化约束

- 不改文件操作命令、右键菜单动作、知识卡筛选规则、技能启停状态和图标 SVG 路径。
- `src/App.tsx` 仍负责跨组件状态、Tauri 调用和主界面布局编排。

## 回退

本轮是前端纯搬迁；如出现回归，可回退本次提交，恢复这些 UI/helper 在 `App.tsx` 内联定义。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，26 个 Rust 测试全部通过。
