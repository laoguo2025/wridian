# 2026-06-10 UI Default Project Cleanup

## 背景

用户指出左侧已有文件夹图标，不应再增加重复“选择文件夹”图标；未选择库时不需要额外“选择文件夹”空态组件；右上角项目下拉不应显示默认示例“雾城手记”。

## 变更

- 移除左侧工具栏新增的重复文件夹切换图标。
- 原有文件夹图标在未选择库时执行选择本地文件夹，已选择库时打开当前库文件夹。
- 移除文件树空态中的选择组件，未选择库时文件树区域保持空白。
- 停止初始化默认示例作品“雾城手记”和默认知识卡示例。
- 未选择作品库时，项目下拉不再从默认 works fallback 派生项目。
- 清理本机运行时缓存中的旧“雾城手记”项目条目和默认示例文件。

## 验证

- `cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- `npm run build` 通过。
- `rg -n "雾城手记|知识卡示例|FolderSwitchIcon|file-tree-empty|选择文件夹" src src-tauri -S` 无命中。

## 回退

- 前端入口集中在 `src/App.tsx` 和 `src/App.css`。
- 默认示例初始化集中在 `src-tauri/src/runtime.rs`。
- 项目下拉派生边界集中在 `src-tauri/src/projects.rs`。
