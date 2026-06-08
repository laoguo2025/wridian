# 2026-06-09 Cocreation Panel

## 背景

执行最小 MVP 的第一步：修正底部共创输入和记忆抽屉混用。旧行为是发送输入后创建记忆候选并打开记忆面板，用户无法区分“共创对话”和“记忆管理”。

## 变更

- 新增 `src-tauri/src/cocreation.rs`：
  - `wridian_cocreate` 复用已保存的 OpenAI-compatible API。
  - 请求上下文包含当前文件、正文片段、active context 和已确认记忆。
  - 返回模型共创回复和本次使用的记忆列表。
- 前端新增“共创”侧边面板：
  - 底部输入发送后打开共创面板。
  - 顶部“记忆”按钮仍打开记忆面板。
  - 普通共创不创建候选记忆，也不自动写长期记忆。
- 记忆模块只暴露最小相关记忆片段读取函数，供共创上下文组装复用。

## 验证

- `npm run build` 通过。
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` 通过。
- 使用 VS Build Tools 环境运行 `cargo check` 通过。
- 使用 VS Build Tools 环境运行 `cargo test --lib` 通过：4 passed，1 ignored。

## 未完成

- 浏览器自动化验证因本地未安装 Playwright 包且 wrapper 路径在当前 shell 下不可用，未完成截图/DOM 自动验收。
- 下一步应做“插入到光标处 / 替换选区 / 复制建议”的回复落地操作。
