# Native Knowledge Health Workflow

## 背景

用户要求把刷新缓存、更新 hot、生成 fold、知识库体检和知识库运维彻底合并为原生“知识库体检”工作流；`zhishiku-skill` 不再作为独立 skill 或 `/` 菜单能力存在。

## 变更

- 删除运行时“知识库运维” creative skill，技能管理和 `/` 菜单只保留作品拆解、知识卡提炼、大神蒸馏。
- 拆分安装包资源：三个创作技能迁到 `resources/skills/work-decompose`、`resources/skills/knowledge-card`、`resources/skills/author-distill`；体检脚本和参考规则迁到 `resources/knowledge-health`。
- 新增原生命令 `wridian_run_knowledge_health_check`：刷新 manifest、更新 `hot.md`、生成 fold、运行健康扫描、写入当天 `00知识库治理/知识库体检-YYYY-MM-DD.md`。
- 新增原生命令 `wridian_fix_knowledge_health_low_risk`：只执行补缺目录、补治理说明、补调用记录台账等低风险确定性修复，并重新生成报告。
- 知识图谱右上角新增“知识库体检”，位于“重置视图”左侧；画布体检/修复期间显示扫描动效和“知识库体检中，报告即将生成”提示。
- 体检完成后在图谱画布显示摘要面板，支持打开报告和一键修复；BM25 检索保留，旧刷新缓存/更新 hot/生成 fold 三按钮移除。

## 边界

- 底层缓存、hot、fold、审计命令仍保留为内部复用能力和兼容命令，但不再作为用户层分散入口。
- 一键修复不改写知识卡语义，不合并、不删除、不归档正式知识卡；这些项目只写入报告待确认清单。
- `zhishiku-skill/SKILL.md` 不再作为运行时资源；体检规则变成原生知识库体检资源。

## 验证

- `npm run build`：通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_ops --lib`：通过，10 个测试。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`：通过，104 个测试。
- `python src-tauri/resources/knowledge-health/scripts/check_knowledge_health_resources.py`：通过。

## 回退

- 回退本次提交可恢复旧 `zhishiku-skill` 总控资源、技能管理体检面板和图谱三枚底层按钮。
- 已生成的 `hot.md`、fold、manifest 和体检报告均为知识库运行产物，可按需删除，不影响源码回退。
