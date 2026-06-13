# 2026-06-14 Chat New Episode File Fallback

## 背景

用户截图显示，在对话中要求“根据第1集剧情，续写第2集，在作品库里新建个文档保存”时，模型把“已新建/已保存”作为普通回复展示，文件树没有正确出现新文件，且存在把新文档正文作为当前第1集 edits 的风险。

## 根因

- E2E mock 共创路径没有复用真实链路的“缺少 fileOperations 后处理”，导致测试无法覆盖模型假成功路径。
- 前端先把模型 edits 交给正文 diff，再执行本地写文件 fallback；文件树写入请求和正文编辑请求的优先级反了。
- 本地 fallback 从用户请求推断集数时命中了第一个“第1集”，没有优先使用“续写第2集”的目标集数。

## 变更

- 后端在文件树请求缺少 `fileOperations` 时，如果回复中还有可作为新文档正文的内容，则清空 edits 和 memories，交给前端本地受限 `writeFile` fallback；如果只是口头声称已保存且没有正文，继续拦截为未执行。
- E2E mock 共创路径复用同一后处理逻辑，避免 mock 测试绕过真实防线。
- 前端在明确新建文档请求且没有文件操作时，先尝试本地写文件 fallback，再决定是否展示正文 diff；此类请求不会把模型 edits 自动落到当前打开稿件。
- 前端剥离“已新建/已保存到 works/xxx”这类操作口吻，只把剩余正文写入新文件。
- 前端集数文件名推断改为优先取动作后的集数；多次出现“第 N 集”时取最后一个目标集数，所以“根据第1集剧情，续写第2集”会写入 `第2集.md`。
- 真实 exe E2E 增加回归：模型返回假成功、无 `fileOperations`、带错误 edits 时，Wridian 应在当前作品文件夹创建 `第2集.md`，不改变第1集，不显示 inline diff。

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：通过，43 个共创测试全绿。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml e2e --lib`：通过。
- MSVC 环境下 `npm run tauri -- build`：通过，生成 0.0.9 MSI 和 NSIS 包。
- `powershell -ExecutionPolicy Bypass -File scripts\e2e-launch.ps1 -DebugPort 9222 -StopExisting` 后执行 `node scripts\e2e-smoke.mjs`：通过。覆盖对话驱动作品库增改删、知识库增改删、假成功续写第2集新建文件、Markdown 表格、划词添加到对话并发送、正文 inline diff。

## 产物

- 安装包：`release/Wridian-0.0.9-x64-setup.exe`

## 回退

- 回退 `src-tauri/src/cocreation.rs` 中本轮新增的 local seed 后处理和 mock 后处理。
- 回退 `src/chat/chatManager.ts` 中新文档 fallback 优先级、操作口吻剥离和集数推断修改。
- 回退 `scripts/e2e-smoke.mjs` 中新增回归和可重复运行等待修正。
