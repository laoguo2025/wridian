# 2026-06-11 混合检索、对话沉淀和写入锁

## 范围

- 升级 Relevant Notes 本地召回。
- 增加对话回复手动沉淀为知识卡草稿。
- 增加 Wridian 控制写入的本地多写入锁。

## 变更

- Relevant Notes 从整文件词项重合升级为分段 BM25 式评分，并继续叠加 wikilink/backlink 权重，返回最佳命中片段。
- 助手回复新增“存为卡”动作，写入知识库 `00知识库治理/对话沉淀/`，标记为待核查草稿卡，不绕过 `zhishiku-skill` 的正式知识卡流程。
- 新增 `src-tauri/src/file_lock.rs`，锁文件位于 Wridian runtime `.wridian/locks/`。
- 写入锁覆盖稿件/知识文件保存、工作区配置、文件树结构操作、项目状态、聊天记录、hot cache、记忆树和模型账号配置。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib` 通过，45 个测试全部通过。
- 内置浏览器打开 `http://127.0.0.1:1421/` 验证页面主结构可渲染；普通浏览器环境下出现 Tauri invoke 缺失错误，属于非 Tauri 容器限制。

## 回退

- 可回退本次提交，删除 `file_lock` 模块、聊天知识卡命令和 Relevant Notes 分段评分逻辑。
- 已有知识库文件不做迁移；新生成的对话沉淀卡位于 `00知识库治理/对话沉淀/`，可按普通知识库文件删除或归档。
