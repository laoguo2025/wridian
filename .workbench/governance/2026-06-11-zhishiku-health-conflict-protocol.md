# zhishiku 体检报告与冲突标记协议

## 背景

用户确认“知识库体检报告”和“冲突 / 不确定性标记”应融入 `zhishiku-skill`，而不是回到 Wridian 主程序内置体检按钮或语义判断。

## 变更

- Wridian frontmatter 协议新增 `review_status/conflicts_with/uncertainty`，并兼容中文字段 `体检状态/治理状态/核查状态`、`冲突对象/冲突卡片`、`不确定性/待核查`。
- 知识图谱只读展示 `zhishiku-skill` 产出的体检状态、冲突和待核查标记，并新增“待核查冲突”治理视图。
- 本机 `zhishiku-skill` 补充体检报告输出和冲突/不确定性标记规范；语义判断仍由 skill 或用户确认产出。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_graph` 通过，覆盖 `zhishiku-skill` 产出字段只读进入图谱。
- `python C:\Users\Administrator\Desktop\zhishiku-skill\scripts\check_zhishiku_skill_quality.py` 通过，`SKILL.md` 保持 420 行，未超过质量门禁。
- `health_check_knowledge_base.py --report` 已用临时知识库验证，可生成 `00知识库治理/知识库体检-YYYY-MM-DD.md` 并汇总冲突/待核查标记。

## 回退

回退 Wridian 图谱字段读取和文档协议即可取消只读展示；回退 `zhishiku-skill` 文档和脚本改动即可恢复旧体检流程。
