# 审查问题 2/3/4/5/7 修复

## 背景

继续处理项目审查中编号 2、3、4、5、7：Tauri CSP 关闭、聊天归档 Markdown 污染、知识图谱和相关稿件无上限扫描、图谱读取错误静默吞掉、模型响应兼容性和配置测试不足。

## 变更

- `tauri.conf.json` 从 `csp: null` 改为最小 CSP，保留 Tauri IPC、asset 和本地 dev server 需要的源。
- 聊天归档将用户输入、模型回复、选区和上下文 pill 内容写入 fenced text block，标题做 Markdown heading 转义。
- 知识图谱扫描增加文件数、深度和单文件大小上限；可跳过问题写入 `warnings` 并在知识图谱弹窗显示。
- 相关稿件召回增加文件数、深度和单文件大小上限，继续对真实读取错误显式失败。
- 对话响应解析兼容 chat completions 字符串、content parts 和 Responses 风格 `output_text`/`output` 文本；自定义 API 测试从只看 HTTP 200 提升为验证响应文本可解析。

## 回退

- CSP 若影响桌面启动，优先仅回退 `tauri.conf.json` 的 CSP 字符串。
- 归档格式如需兼容旧解析，可在 `chat_persistence.rs` 中保留 fenced 内容并新增结构化旁路，不建议恢复直接拼接正文。
- 扫描门禁如过严，可调整常量上限，不应移除上限。
- 模型响应兼容可独立回退 `read_model_response_text` 扩展逻辑。

## 验证

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，26 个测试全绿。
- 旧风险关键词复查：无 `csp: null`、知识图谱 `filter_map(Result::ok)`、旧 `choices[0].message.content` 错误文案或旧“连接成功。”后端测试语义残留。
