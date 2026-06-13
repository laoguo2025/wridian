# 2026-06-13 file operation claim repair

## 问题

用户要求“根据第1集剧情，续写第2集，放在新建文档里”时，模型可能只在 `reply` 中声称“剧本已新建为 `works/第2集.docx`”，但 `fileOperations` 为空。此时 Wridian 没有可执行文件树操作，左侧文件树不会出现新文件，形成假成功。

## 变更

- 协同对话模型返回后，若满足“用户明确要求新建/写入文件、`fileOperations` 为空、`reply` 声称已新建或已写入文件”，后端会用同一上下文自动补救请求一次，要求模型只返回可执行 JSON 文件操作。
- 补救请求拿到 `fileOperations` 后继续走既有路径路由、写入、审计和文件树刷新链路；用户正在打开作品文件时，新建作品文件仍默认落到当前文件所在文件夹。
- 补救仍失败时，Wridian 会拦截假成功回复，提示没有写入任何文件，并清空本轮 edits/memories，避免把未执行结果沉淀为记忆。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `npm run build`
- `npm run tauri -- build`

## 产物

- `release/Wridian-0.0.8-test.exe`，SHA256 `A90F084E1C35C1358B55C6CAF7BE9F372E0D1A91122D2825F7D6E74361B25D3F`
- `release/Wridian-0.0.8-x64-setup.exe`，SHA256 `9419E2AA369F7460869F790E9725D551874072889B0D6A4BD42B91F9712E7804`

## 回退

回退 `src-tauri/src/cocreation.rs` 中的补救请求、假成功拦截判定和对应测试，并移除此治理记录及项目地图新增句。
