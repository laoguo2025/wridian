# P6 知识缓存、检索与 Fold

## 目标

解决 P6：知识库 hot cache、manifest 增量、BM25 检索、fold 压缩。

## 变更

- 新增 `src-tauri/src/knowledge_ops.rs`：
  - `wridian_refresh_knowledge_cache` 写入知识库 `.wridian/knowledge-manifest.json`。
  - `wridian_search_knowledge_bm25` 基于 Metadata Index 和 Markdown 内容做知识域 BM25 检索。
  - `wridian_update_knowledge_hot_cache` 生成知识库 `hot.md`。
  - `wridian_fold_knowledge_cache` 在 `00知识库治理/folds/` 生成抽取式 fold。
- manifest 增量规则：当相对路径、mtime 和长度未变时，复用旧 manifest 的 `sha256` 与 token 统计；关系计数仍来自最新 Metadata Index。
- 知识图谱抽屉新增 P6 操作区，可刷新缓存、更新 hot、生成 fold、执行 BM25 搜索并打开结果文件。
- 更新 Metadata Index 长期口径，明确知识缓存层仍以 Metadata Index 为事实源。

## 边界

- P6 仅作用于知识库；作品库仍由记忆树和相关稿件链路服务。
- 生成产物限定在知识库 `.wridian/knowledge-manifest.json`、`hot.md`、`00知识库治理/folds/`。
- fold 为抽取式压缩，不新增事实。
- 本轮未接入远程 embedding；语义检索保留为后续本地/provider 层扩展。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_ops --lib`：通过，4 个测试。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`：通过，89 个测试。
- `npm run build`：通过。

## 回退

- 回退本次提交即可移除 P6 命令、前端入口和文档更新。
- 已生成的知识库 `.wridian/knowledge-manifest.json`、`hot.md`、`00知识库治理/folds/*` 是运行产物，可按需删除。
