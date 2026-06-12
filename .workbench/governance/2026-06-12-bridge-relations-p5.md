# P5 作品域和知识域显式桥接

## 目标

补齐作品库和知识库之间的显式桥接动作。桥接只写入 frontmatter 关系字段，不复制知识卡到创作记忆树，不自动把作品内容沉淀为知识卡。

## 变更

- 新增后端命令 `wridian_apply_bridge_relation`，目标文件必须是对应库根目录内的 Markdown 文件，且不能是符号链接或越界路径。
- 后端按协议字段写入并去重：
  - 作品文件引用知识：`references_knowledge`、`adopts_knowledge`、`derived_from_knowledge`。
  - 知识文件抽象作品：`abstracted_from_draft`。
  - 后端保留 `excerpted_from_project` 和 `distilled_from_memory` 的动作映射，前端第一版不暴露。
- 前端在当前打开 Markdown 文件和对话上下文 pill 存在另一域 Markdown 文件时显示桥接按钮。
- 点击桥接前先保存当前文件，写入后重新打开当前文件以同步 frontmatter。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml bridge --lib`
- `npm run build`

## 回退

回退本次提交即可移除桥接命令和 UI。已写入用户 Markdown 的 frontmatter 关系字段是普通文本，可由用户在文件编辑区手工删除。
