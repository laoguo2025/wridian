# Skill 安装策略

用于安装 `08大神蒸馏` 中的作者小 skill。

## 安装来源

```text
{知识库}/08大神蒸馏/{作者名}/{skill-name}/
```

该目录必须包含：

```text
SKILL.md
```

可选：

```text
references/
scripts/
版本记录.md
```

## 安装目标

安装目标不是固定路径，而是由运行环境决定：

```text
Claude Code 默认：~/.claude/skills/{skill-name}/
其他 Agent：{用户指定的 skill 根目录}/{skill-name}/
```

执行安装时使用 `--target-root` 显式传入目标根目录：

```bash
python3 {skill_dir}/scripts/install_skill.py \
  --source {知识库}/08大神蒸馏/{作者名}/{skill-name} \
  --target-root {目标skill根目录} \
  --knowledge-root {知识库}
```

如果用户使用的不是 Claude Code，不要假设存在 `~/.claude/skills/`；先获得该工具的 skill 根目录。

## 安装规则

1. 目标不存在：直接复制安装。
2. 目标存在：先备份，再覆盖。
3. 覆盖已安装 skill 前，必须获得用户确认。
4. 安装后更新：

```text
{知识库}/08大神蒸馏/_安装记录.md
```

## 备份路径

```text
{知识库}/09文件归档/skill备份-YYYY-MM-DD/{skill-name}/
```

## 可分发要求

作者小 skill 不得依赖用户本地路径，例如：

```text
/Users/mac/Desktop/Wridian知识库/02拆解报告/...
```

应在 `references/` 中保存必要证据摘要或案例片段。
