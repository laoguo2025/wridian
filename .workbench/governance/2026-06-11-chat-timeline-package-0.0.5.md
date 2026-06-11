# 2026-06-11 对话时间线修复后打包 0.0.5

## 范围

- 清理 `src/settings/ModelSettingsDialog.tsx` 换行符脏标记。
- 基于项目级对话隔离、对话时间线、上下文命中提示中文化后的当前版本重新打包 0.0.5。

## 产物

- 测试版 exe：`release/Wridian-0.0.5-test.exe`
- 正式安装包：`release/Wridian-0.0.5-x64-setup.exe`

## 校验

- `npm run build` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib` 通过，68 个测试全部通过。
- `cmd.exe /d /s /c "<vcvars64.bat> >nul && cd /d D:\Coding\Wridian && npm run tauri -- build"` 通过，生成 MSI 和 NSIS 包。
- `release/Wridian-0.0.5-test.exe` SHA256：`149FC8F6FF001D46A09217320557FA931D913291DD9021DE3431DDC4846BEEBB`
- `release/Wridian-0.0.5-x64-setup.exe` SHA256：`55A9791F56D9368F865FD6F71DFD3AAF72DAFC16EE0065EF996BADDCDB5C9971`
