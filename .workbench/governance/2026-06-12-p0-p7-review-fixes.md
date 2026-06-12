# P0-P7 review fixes

## 原因

修复 P0-P7 review 中确认的行为问题：Relevant Notes 被不可读候选拖垮、作品域和知识域自动混检、Wridian 生成的 hot/fold 回流污染图谱和知识检索，以及桥接动作在 UI 中未完整暴露。

## 变更

- Relevant Notes 只接受作品库来源，并只扫描作品库候选；候选读取改用工作区文本读取链路，单个坏文件或 docx 读取失败只跳过。
- 知识缓存、BM25、体检和知识图谱排除 `wridian_generated` / `knowledge_hot_cache` / `knowledge_fold` 生成文件；`hot.md` 不再用 wikilink 指向 JSON manifest。
- 知识目标文件在已有作品 pill 时提供“从作品抽象 / 摘录到项目”，在已有记忆 pill 时提供“从记忆蒸馏”。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml`

## 回退

回退本次提交即可恢复 review 前行为；本轮没有修改外部配置、发布产物或用户数据。
