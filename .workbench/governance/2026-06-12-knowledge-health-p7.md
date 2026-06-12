# P7 知识库体检与 skill 化成熟度

## 范围

- 落地知识库健康检查命令和技能管理面板入口。
- 体检范围只针对知识库，复用 Metadata Index 与知识 manifest，不新增独立扫描器。
- 评分覆盖断链、frontmatter、标签、来源线索、孤立知识卡、正式 skill 文件和 skill 化候选成熟度。

## 变更

- 后端新增 `wridian_audit_knowledge_health`，返回健康分、skill 化成熟度、统计摘要、问题列表和候选列表。
- 前端技能管理抽屉新增“知识库体检”区域，支持手动运行体检并展示关键统计、主要问题和 skill 化候选。
- TypeScript 增加知识体检返回类型。
- Rust 增加回归测试，覆盖断链降分、来源/结构缺失降成熟度、正式 skill 识别与候选评分。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_ops`
  - 通过：7 passed。
- `npm run build`
  - 通过：TypeScript 与 Vite 构建成功。

## 回退

- 可回退本次提交；体检命令为新增能力，不改变现有知识图谱、知识缓存、BM25 或 fold 生成链路。
