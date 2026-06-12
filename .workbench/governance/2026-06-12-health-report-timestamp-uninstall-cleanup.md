# 2026-06-12 health report timestamp and uninstall cleanup

## Scope

- Fix knowledge health report filenames that still used Unix seconds, such as `知识库体检-1781276932.md`.
- Extend uninstall data cleanup so generated health reports and related runtime files are removed when the user checks "delete app data".
- Rebuild the formal Windows x64 installer.

## Changes

- `iso_timestamp()` now returns local RFC3339 seconds instead of Unix seconds.
- Added separate runtime helpers for filename-safe timestamps and Unix seconds.
- Knowledge health reports now use `知识库体检-YYYYMMDDTHHMMSS.md`.
- Trash and duplicate-name fallback paths use filename-safe timestamps.
- NSIS uninstall hook now clears:
  - `$APPDATA\Wridian`
  - `$LOCALAPPDATA\Wridian`
  - default `D:\Wridian知识库`
  - configured knowledge-root runtime artifacts from `workspace.json`
  - common `Wridian知识库` runtime artifacts under user profile/documents
  - known Wridian model-provider Windows Credential Manager targets

## Verification

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test timestamps_distinguish_iso_filename_and_unix_seconds --manifest-path src-tauri\Cargo.toml`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test health_report_timestamp_uses_date_time_filename --manifest-path src-tauri\Cargo.toml`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed.
- `npm run build`: passed.
- `npm run tauri -- build` through Visual Studio Build Tools `vcvars64.bat`: passed after switching uninstall cleanup to an encoded PowerShell command.
- Encoded uninstall cleanup command was decoded and inspected; it reads `workspace.json` before removing only Wridian runtime artifacts inside the configured knowledge root.

## Package

- Previous installer backup: `.workbench/runtime/release-backups/Wridian-0.0.7-x64-setup-20260612-232802-prev.exe`.
- Final installer: `release/Wridian-0.0.7-x64-setup.exe`
  - Size: `4531935`
  - SHA256: `2EAD768ED2DEB309FD35509846B69A90031778BEB99FA599A17AF3F636753843`
- Final runtime exe: `src-tauri/target/release/wridian.exe`
  - ProductVersion: `0.0.7`
  - FileVersion: `0.0.7`
  - SHA256: `5706DCEBC84530126F2E63444B656E3CD8284A0A0A98A1AD6CF2F1FA7BEEE3E4`

## Rollback

- Revert this task commit to restore Unix-second timestamp behavior and the previous uninstall cleanup hook.
- Restore the backed-up installer to `release/Wridian-0.0.7-x64-setup.exe` if only the package artifact needs rollback.
