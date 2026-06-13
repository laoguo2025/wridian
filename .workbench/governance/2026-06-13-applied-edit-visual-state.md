# 2026-06-13 applied edit visual state

## Scope

- Make automatically applied draft edits visible in the editor after the text has already been changed.

## Root Cause

- Explicit edit requests auto-applied safe edits directly to `editorContent` and only retained the undo snapshot.
- The inline diff renderer only displayed pending edits, so users could see "撤销修改" but not where the draft changed.

## Changes

- Added an applied-edit visual state separate from pending edits.
- Auto-applied and manually accepted edits now record replacement ranges after the content change.
- The draft editor renders applied replacements with a green highlight and keeps the original text in the hover title.
- Pending and applied diff displays do not show rationale text inline; the editor should emphasize the changed text itself.
- Chat edit requests default to the currently open file. When no edit can be safely located in that file, the chat error asks the user to confirm the correct file is open.
- Manual typing, file switching, and undo clear the applied highlight state.
- Updated the project map with the durable visual-state behavior.

## Verification

- `npm run build`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed.
- `cmd.exe` with Visual Studio `vcvars64.bat`, then `npm run tauri -- build`: passed.
- Updated local artifacts:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `8762A2238BE2F66929ECB97A1719754F32386DBF7278FF9C3A4FC9B3B6F5E2F4`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `01ED5EB4C9D36F93DF162C70BB801069948D1169F94998374504EBC9EC4C50BD`

## Rollback

- Revert this task commit to restore the prior behavior where auto-applied edits only exposed "撤销修改".
