# 当前文件夹文件树写入修复

## 背景

用户在作品库中打开 `测试/第1集.docx`，要求“新建一个 md 文件，根据第1集剧情续写第2集”。模型返回 `fileOperations.writeFile` 时给出 `works/第2集.md`，旧行为会按作品库根路径处理，或者在作品库配置缺失时回落到 Wridian 默认 `vault/works`。用户期望新文件出现在当前打开的本地作品文件夹中。

## 变更

- 协作执行层在当前打开文件属于作品库时，会把模型给出的单文件名或冗余 `works/文件名` 路径路由到当前文件所在文件夹。
- 模型文件树操作执行前检查作品库是否已显式绑定且目录仍存在；未绑定时拒绝写入，不再回落到默认 `works` 根目录。
- 对话回复中的文件操作结果改用实际执行路径生成，避免继续显示模型原始的 `works/第2集.md`。

## 验证

- `new_work_file_routes_to_current_open_file_folder`：通过，覆盖当前打开 `测试/第1集.docx` 时将 `works/第2集.md` 写到 `测试/第2集.md`。
- `model_file_operation_rejects_unconfigured_work_root`：通过，覆盖未绑定作品库时拒绝写默认根。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`：36 个 cocreation 测试通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`：通过。
- `npm run build`：通过。
- 通过 Visual Studio Build Tools `vcvars64.bat` 后 `npm run tauri -- build`：通过。
- 已复制本地测试版：`release\Wridian-0.0.8-test.exe`，SHA256 `F0EA06D33D70F5D89C461478D5BE44F9F4D6A26BD319B898253CDEDD828CA12C`。
- 已复制 NSIS 安装包：`release\Wridian-0.0.8-x64-setup.exe`，SHA256 `9AB91CEF3344681894A78166E247E3A80A4FBF830EF1CFB7F042E78D37A91631`。
