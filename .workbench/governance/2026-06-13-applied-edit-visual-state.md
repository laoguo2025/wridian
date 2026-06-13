# 2026-06-13 inline diff visual state correction

## Scope

- Restore the expected inline diff review state for chat-driven draft edits.

## Root Cause

- The previous applied-edit visual state wrote safe edits directly into `editorContent` and rendered only a green applied highlight.
- That removed the expected red deletion, green insertion, and per-edit confirm/cancel buttons from the draft editor.

## Changes

- Chat edit requests still default to the currently open file.
- Safe matches now stay as pending edits and render in the draft editor as inline diff: red deletion, green insertion, and confirm/cancel buttons.
- No direct write happens until the user confirms one edit or all edits.
- Removed the applied green-only highlight state.
- When no edit can be safely located in the current file, the chat error asks the user to confirm the correct file is open.
- Updated the project map with the durable inline diff behavior.

## Verification

- `npm run build`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed.
- `cmd.exe` with Visual Studio `vcvars64.bat`, then `npm run tauri -- build`: passed.
- Updated local artifacts:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `AA9CE099CB287EF9A263BF5997374C0A49132C14530F5B3538FCEA730D209401`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `D15811A58E7E423DE1E352578800FF7F0E3AB26883EFCF8F5C7983B51F11561F`
- Follow-up correction after user screenshot: restore pending inline diff as the visual state before confirmation.

## Rollback

- Revert the correction commit to return to the green-only applied highlight behavior, which is not the desired UX.
