# Creative Skills P4 Workflows

## 目标

解决 P4：四个内置技能不能只作为知识卡说明或提示词入口，必须具备可执行工作流约束，覆盖输入、产物、写入、质检和回滚。

## 现场判断

- 技能来源为安装包内置资源 `src-tauri/resources/skills/zhishiku-skill/`。
- `/` 选择技能后进入对话的 `tool` 槽位，后端会展开内置技能文件。
- 对话实际落地产物必须走既有 `fileOperations`，不能新增孤立执行器。

## 变更

- 前端技能定义增加 workflow 元数据：输入、产物、质检和回滚。
- 技能管理抽屉展示 workflow 摘要，用户可在启用前看到执行边界。
- 后端对话 prompt 增加技能工作流协议，要求带技能规则时按可执行流程输出，并通过 `fileOperations` 写入库内相对路径。
- 四个内置技能资源补充 Wridian P4 可执行工作流契约。

## 回退依据

如需回退，恢复 `src/creativeSkills.ts`、`src/skills/CreativeSkillsDrawer.tsx`、`src/App.css`、`src-tauri/src/cocreation.rs`、`src-tauri/resources/skills/zhishiku-skill/SKILL.md` 和三个 `references/embedded-skills/*-skill.md` 的本次改动即可。不涉及用户数据迁移。

## 验证

- 通过：`npm run build`
- 通过：`powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml build_prompt_separates_tool_protocol_from_explicit_context_items`
- 通过：`powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
