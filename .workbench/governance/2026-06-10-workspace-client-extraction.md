# 工作区客户端拆分留痕

## 目标

继续降低 `src/App.tsx` 的命令字符串和 helper 堆积，把可复用的工作区文件 Tauri 命令集中到前端客户端模块。

## 变更

- 新增 `src/workspace/workspaceClient.ts`，封装工作区初始化、根目录选择、文件打开/保存、新建、复制、重命名和移到回收站命令。
- 新增 `src/editor/draftKind.ts`，承载稿件 basename 和剧本/散文类型识别。
- `src/App.tsx` 保留状态流、错误展示和跨组件编排，不再直接散落工作区文件命令字符串。

## 非变化约束

- 不改 Tauri 命令名、命令参数、自动保存时机、打开文件后的状态清理、项目切换和提示文案。
- 不改稿件类型识别正则和阈值。

## 回退

本轮是前端封装搬迁；如出现回归，可回退本次提交，恢复 `App.tsx` 直接调用 Tauri 命令。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，26 个 Rust 测试全部通过。
