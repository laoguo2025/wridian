# 2026-06-11 OpenHuman Creative Memory Tree Continuity

## 背景

用户要求继续借鉴 OpenHuman 的创作记忆树，但排除作品元素和 World Info。本轮目标是增强 Wridian 已有 Markdown 记忆树的关系可视化和作品分支续接。

## 本轮适配

- 记忆树后端递归展示 `leaves/drama` 和 `leaves/novel` 下的作品项目记忆分组，`project.md` 与 `compressed.md` 作为可编辑核心记忆出现在树里。
- 创作记忆树抽屉增加作品项目过滤，只按项目 `source:` 匹配作品连续性记忆，不把知识库内容作为项目过滤结果。
- 项目核心记忆点与普通叶子点在画布上区分展示，编辑器标题区说明分支、所属项目和记忆角色；`project.md` 和 `compressed.md` 仍只能编辑不能删除。
- Project Mode 续接上下文从只读 `compressed.md` 扩展为读取 `project.md`、`compressed.md` 和少量同项目必要叶子。

## 不搬内容

- 未引入 OpenHuman 的通用来源注册、托管登录、搜索代理、个人 AI OS 记忆源和通用知识图谱能力。
- 知识卡仍只通过 `@` 或相关内容显式进入对话，不默认混入作品续接记忆。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml memory_tree --lib` 通过，覆盖项目核心记忆可见和作品续接不混入知识库。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml` 通过。
- 尝试启动 Vite 并做浏览器烟测；当前会话未暴露 Browser 工具，Node REPL 也缺少 Playwright 包，未完成点击截图验收。

## 回退依据

- 续接上下文入口集中在 `src-tauri/src/projects.rs` 的 `read_active_project_context`。
- 记忆树读取与项目续接文件收集集中在 `src-tauri/src/memory.rs`。
- 前端过滤和视觉区分集中在 `src/memory/MemoryDrawer.tsx` 和 `src/App.css`。
