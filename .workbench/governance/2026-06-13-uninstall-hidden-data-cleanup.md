# 2026-06-13 uninstall hidden data cleanup

## Scope

- Fix uninstall cleanup so selecting "delete app data" does not show a PowerShell console window.
- Make "delete app data" remove Wridian chat/session data and the default auto-created knowledge base content that reappeared after reinstall.

## Root Cause

- `src-tauri/nsis-hooks.nsh` used `ExecWait` to run `powershell.exe`; NSIS can show a visible console for that child process during uninstall.
- The cleanup script removed Wridian runtime artifacts inside knowledge roots but did not remove the default `D:\Wridian知识库` root itself. Reinstall seeded or reused that same default root, so user-created Markdown files were still visible.
- Chat history lived under `$APPDATA\Wridian\.wridian\chat` and related runtime files. Cleaning the full Wridian data directory is the correct app-data deletion behavior.

## Changes

- Added `.workbench/tools/generate-nsis-hooks.mjs` as the owner for regenerating `src-tauri/nsis-hooks.nsh`.
- Switched uninstall child commands from `ExecWait` to `nsExec::ExecToLog`, with PowerShell `-NonInteractive -WindowStyle Hidden` only for custom knowledge-root cleanup.
- Cleanup now removes:
  - `$APPDATA\Wridian`
  - `$LOCALAPPDATA\Wridian`
  - `$APPDATA\ai.wridian.app`
  - `$LOCALAPPDATA\ai.wridian.app`
  - default auto-created `Wridian知识库` roots under `D:\`, user profile, and Documents
  - known Wridian model-provider Windows Credential Manager targets
- Fixed app-data and default-knowledge-root deletion uses NSIS `RMDir /r`; a user-selected custom knowledge root from `workspace.json` remains conservative and removes only Wridian-generated runtime artifacts.

## Verification

- Decoded `src-tauri/nsis-hooks.nsh` encoded PowerShell command and inspected cleanup targets: passed.
- Regenerated hook from `.workbench/tools/generate-nsis-hooks.mjs` and inspected key checks: passed.
- `npm run build`: passed.
- `cmd.exe` with Visual Studio `vcvars64.bat`, then `npm run tauri -- build`: passed; NSIS installer built successfully.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed.
- Updated local artifacts:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `82CA1B9BEFB9E1387D116C4E45C1B6FF7DB4B0954E6B4396C7118DD35BB91250`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `4103442E09A49F7A64435B4BDCB87821FD01362C3781B1063A358049747CA05B`

## Rollback

- Revert this task commit to restore the previous uninstall hook.
- If only packaging needs rollback, rebuild from the previous committed hook or restore the prior installer artifact.
