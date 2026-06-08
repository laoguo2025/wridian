# 2026-06-09 MVP Planning

## 背景

复盘 `C:/Users/Administrator/Desktop/000.md` 后，确认 Wridian 的长期定位不能只按小说编辑器推进，也要覆盖短剧剧本、剧本、分集大纲、场景稿、人物小传和设定资料。

当前发现一个产品语义问题：底部共创输入现在直接创建记忆候选并打开记忆抽屉，导致“共创对话”和“记忆管理”混用。

## 更新

- 更新 `.workbench/doc/wridian-project-map.md`：
  - 明确 Wridian 是本地写作共创系统，不是通用 AI OS 或小说专用编辑器。
  - 增加交互边界：稿件编辑区、共创输入、共创侧边面板、记忆侧边面板分开。
  - 增加小说与短剧/剧本作品类型边界。
  - 记录借鉴项目边界：obsidian-copilot、claude-obsidian、OpenHuman、holaOS、SillyTavern、Beat/Fountain、Twine/ink/Yarn Spinner 等。
  - 增加最小 MVP 路线。

## MVP 顺序

1. 修正共创输入和记忆抽屉混用。
2. 实现最小共创请求：当前文件、active context、已确认相关记忆和用户输入一起发给模型。
3. 支持把 AI 回复安全插入或替换选区。
4. 让已确认记忆参与共创上下文，普通共创不直接写记忆。
5. 补剧本 MVP：`.fountain`、对白、冲突、钩子、角色口吻、分集节奏。

## 验证

本轮只更新项目治理文档和规划，不改功能代码。
