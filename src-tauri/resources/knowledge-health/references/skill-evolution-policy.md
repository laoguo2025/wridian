# Skill 进化策略

体检功能包含进化大 skill 和小 skill。

## 大 skill

检查对象：

```text
chaijie-skill
tilian-skill
zhengliu-skill
```

检查项：

- description 是否覆盖触发词。
- 工作流是否明确。
- 输入/输出路径是否清晰。
- 是否存在本地绝对路径依赖。
- 是否与当前知识库目录冲突。
- 是否有失败处理与归档策略。

## 小 skill

检查对象：

```text
{知识库}/08大神蒸馏/*/*/
{当前环境的skill根目录}/*
```

Claude Code 默认 skill 根目录为 `~/.claude/skills/`。其他 Agent 使用用户提供或工具文档明确的 skill 根目录。

检查项：

- `SKILL.md` 是否存在。
- description 是否可触发。
- 是否有心智模型。
- 是否有证据索引。
- 是否有诚实边界。
- 是否有调用协议。
- 是否自包含、可分发。

## 可自动进化

低风险项可自动应用：

- 补 description 触发词。
- 补缺失目录说明。
- 修正本地绝对路径为相对说明。
- 补版本记录和安装记录。
- 补格式字段。

## 需确认进化

高风险项必须列清单等待用户确认：

- 重写核心心智模型。
- 删除旧模型。
- 覆盖已安装 skill。
- 合并两个作者 skill。
- 批量更新当前环境的 skill 根目录。Claude Code 为 `~/.claude/skills/`；其他 Agent 使用用户指定根目录。
