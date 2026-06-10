# 自动记忆叶子写入

## 背景

审查发现候选记忆叶子链路只有后端命令、前端状态和候选 UI，没有任何触发入口，属于不可达功能。当前产品口径调整为：不需要候选确认；对话 agent 调用模型分析后自动提取长期记忆，写入后由用户在创作记忆树中查看、编辑和删除。

## 变更

- 对话模型输出 JSON 增加 `memories` 数组。
- 后端在 `wridian_cocreate` 成功解析模型回复后，将结构化记忆写入 `.wridian/memory-tree/leaves/<branch>/`。
- 移除前端候选叶子状态和候选确认面板。
- 创作记忆树文件编辑器增加删除动作；后端只允许删除 leaves 下的普通 Markdown 叶子文件，并拒绝删除项目 `project.md` 与 `compressed.md`。
- 项目地图同步更新为自动写叶、用户编辑删除的长期口径。

## 回退

- 若自动沉淀误写过多，可临时让模型提示词固定 `memories: []`，或撤回 `wridian_cocreate` 中 `write_memory_leaves` 调用。
- 已写入的普通叶子文件可从创作记忆树删除；项目核心记忆文件不允许通过删除按钮移除。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，21 个测试全绿。
- 旧候选链路关键词搜索无命中：`MemoryLeafCandidate`、`wridian_propose_memory_leaf`、`wridian_plant_memory_leaf`、候选 UI class 与候选确认文案均已清理。
