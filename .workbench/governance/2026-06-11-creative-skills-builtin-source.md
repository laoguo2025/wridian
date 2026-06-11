# 技能来源内置化修复

## 背景

技能管理面板把“知识库运维”的来源显示为本机桌面 `zhishiku-skill/SKILL.md`。这说明运行时把开发机上的外部 skill 文件当作能力来源，不符合发布给其他用户时的可分发要求。

## 根因

`wridian_get_creative_skill_sources` 会扫描桌面、`.codex/skills` 和 `.agents/skills` 下的 `zhishiku-skill/SKILL.md`，找到后把绝对路径返回给前端展示。前端实际发送给模型的技能提示词来自应用内常量，因此上一版只隐藏路径或只返回逻辑 `builtin` 仍不够，发布版必须随安装包携带真实 `SKILL.md`、`references/` 和 `scripts/`。

## 变更

- 将桌面 `zhishiku-skill` 安装到 `src-tauri/resources/skills/zhishiku-skill/`，纳入主 `SKILL.md`、`references/` 和 `scripts/`，排除 `__pycache__` / `.pyc` 本机缓存。
- `tauri.conf.json` 增加 `bundle.resources`，打包时把 `resources/skills/zhishiku-skill/**/*` 复制到安装包资源目录。
- 技能来源接口通过 Tauri `BaseDirectory::Resource` 解析安装后的 `resources/skills`，为四个技能分别返回内置资源文件路径：
  - 知识库运维：`zhishiku-skill/SKILL.md`
  - 作品拆解：`zhishiku-skill/references/embedded-skills/chaijie-skill.md`
  - 知识卡提炼：`zhishiku-skill/references/embedded-skills/tilian-skill.md`
  - 大神蒸馏：`zhishiku-skill/references/embedded-skills/zhengliu-skill.md`
- `/` 选择技能时，tool pill 携带对应内置资源路径；后端只允许 `tool` 类型读取安装包 `resources/skills` 白名单内文件，读取后的技能文件正文进入 `skill 协议` 槽位。
- 技能管理面板只显示技能名和一句用途介绍，不展示 `zhishiku-skill` 接入文案或资源来源；内置资源路径仅保留给 `/` 调用链路使用。

## 回退

如需回退，恢复 `src-tauri/resources/skills/zhishiku-skill/`、`src-tauri/tauri.conf.json`、`src-tauri/src/creative_skills.rs`、`src-tauri/src/cocreation.rs`、`src/App.tsx`、`src/appTypes.ts`、`src/chat/promptContext.ts`、`src/creativeSkills.ts`、`src/skills/CreativeSkillsDrawer.tsx` 的本次改动即可。不涉及用户数据迁移。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml creative_skill_sources_are_builtin_resources_and_distributable` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml expands_builtin_skill_resource_for_tool_context` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml` 通过。
- `cargo fmt --manifest-path src-tauri\Cargo.toml --check` 仍失败在既有 `src-tauri/src/chat_persistence.rs` 格式差异，本轮未改该文件。
- 资源目录检查：`src-tauri/resources/skills/zhishiku-skill/` 共 25 个文件，未纳入 `__pycache__` 或 `.pyc`。
