# Knowledge Graph P3 Metadata

## 目标

解决 P3：知识图谱只服务知识库，并补齐可见的反链、断链、别名、标签和来源引用信号。

## 变更

- 复用 `metadata_index` 作为唯一解析底座，没有新增独立 Markdown 解析器。
- 知识图谱节点增加 aliases、tags、source refs、出链数、反链数、断链数和被引用来源。
- `unresolved_links` 进入图谱，未解析 wikilink/frontmatter/embed 关系生成断链节点和断链边。
- 前端图谱悬浮预览展示元数据；断链节点只展示元数据，不读取或打开文件。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_graph --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml metadata_index --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 fmt --manifest-path src-tauri\Cargo.toml`

## 回退

回退本次提交即可恢复到 P3 前的知识图谱字段、断链节点和前端预览行为。
