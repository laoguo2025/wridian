# 2026-06-11 对话点名文件权限修正

## 背景

用户明确纠正右侧对话区语义：不是自动读取文件并展示“相关内容”，而是在用户与 Wridian 对话时，若谈及作品库或知识库文件树中的文件，Wridian 有权限在当前库内查看内容、新建文件、修改文件/文件名、删除文件（移到回收站）。

## 变更

- 移除前端对话区自动调用相关内容召回和固定展示“相关内容”的路径。
- 保留文件树相对路径进入对话请求，供模型理解当前作品库/知识库可操作对象。
- 后端新增“点名文件”读取：只有用户输入明确匹配库内相对路径、`works/` / `knowledge/` 前缀路径、完整文件名，或长度足够的文件名主干时，才读取文本可预览文件内容进入本轮上下文。
- 文件操作仍必须走受限 `fileOperations` 协议，限制在当前作品库/知识库相对路径内；删除语义仍为移到 `.wridian-trash/`。

## 回退

回退本轮涉及文件即可恢复旧行为：`src/App.tsx`、`src/chat/ChatPanel.tsx`、`src/chat/projectContext.ts`、`src/App.css`、`src-tauri/src/cocreation.rs`、`.workbench/doc/wridian-project-map.md`。

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：通过，23 项 cocreation 测试通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 fmt --manifest-path src-tauri\Cargo.toml --check`：通过。
- 本地 Vite 服务 `http://127.0.0.1:5173` 指向当前项目入口；当前 Codex Browser 工具未暴露，未做截图烟测。
