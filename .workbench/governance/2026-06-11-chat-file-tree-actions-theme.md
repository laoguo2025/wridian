# 2026-06-11 对话文件树操作与主题持久化

## 范围

- 重装未删除用户数据时，主题和字号应沿用重装前设置。
- 对话需要读取作品库和知识库文件树。
- 对话需要能通过受控协议增、改、删文件树节点。
- 右键文件加入对话时，PDF、图片等非文本文件不能触发 UTF-8 读取错误。

## 变更

- 前端把主题和字号写入 `localStorage`，启动时先读取持久化值。
- 对话后端在每轮请求中编译作品库和知识库的相对文件树；不把本机绝对路径发给模型。
- 新增 `fileOperations` 模型输出协议，支持 `writeFile/createFolder/rename/trash`，仅允许 `works` 或 `knowledge` 内相对路径；后端执行后把结果返回前端。
- 前端收到成功文件操作后刷新工作区文件树，并把执行结果附加到助手回复中。
- 右键加入对话时，`md/txt/docx/csv/json/yaml/yml` 才抽取文本；PDF、图片和 Office 旧格式只作为文件引用加入，由后端在上下文中明确说明当前格式不可抽文本。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml file_tree --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml parsed_file_operations --lib`
- 后续收口前运行完整 `cargo-msvc.ps1 check` 和 `cargo-msvc.ps1 test --lib`。

## 回退

- 回退本提交即可恢复到只读对话上下文和前端内存主题状态。文件操作通过后端安全路径执行，不改写 Git 历史；已由模型实际创建或修改的用户文件需要按用户意图单独处理。
