# Low Priority Debt Fixes

## 原因

继续收口低优先级问题和技术债：脚手架残留、假交互控件、上下文展示条件偏窄、禁用态视觉不一致，以及少量错误被静默吞掉。

## 变更

- 清理 Vite/Tauri 模板标题、favicon 引用和未使用 SVG 资源。
- README 改为 Wridian 项目说明和当前验证命令。
- 右侧对话底部模型显示从不可用的下拉框改成只读状态文本。
- 用户消息只要有上下文 pill 就展示上下文行，不再只在存在选中文本时展示。
- 补全禁用按钮的 hover/disabled 视觉，避免保存中或不可用动作仍像可点击。
- 对话请求和自定义 API 测试读取 HTTP 响应体失败时返回明确错误，不再当空响应处理。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml`

## 回退

如需回退，可恢复模板资源、旧模型下拉和旧响应读取逻辑；但会重新引入脚手架残留、假交互和错误信息缺失。
