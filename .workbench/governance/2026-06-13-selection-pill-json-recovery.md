# 2026-06-13 selection pill json recovery

## Scope

- Make the selected-text "添加到对话" action appear reliably after dragging a draft selection.
- Prevent malformed structured model output from breaking chat sends when a selection pill is included.

## Root Cause

- Selection action placement relied mainly on editor-local key/mouse events, which can miss the final WebView selection state after drag selection.
- Some providers return JSON-looking cocreation output with malformed string escaping. The parser treated any extractable but malformed JSON as a hard chat error.

## Changes

- Added document-level `selectionchange`, `pointerup`, and `keyup` listeners, with a requestAnimationFrame-delayed read of the current draft selection.
- Reused the same delayed selection refresh path from the draft editor.
- Malformed structured output now recovers a visible `reply` when possible and degrades to a plain chat response with no edits/file operations/memories.
- Added parser tests for broken JSON payloads and unescaped quotes in `reply`.
- Updated the project map with the durable selection action behavior.

## Verification

- `npm run build`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test parse_cocreation_model_output --manifest-path src-tauri\Cargo.toml`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed.
- `cmd.exe` with Visual Studio `vcvars64.bat`, then `npm run tauri -- build`: passed.
- Updated local artifacts:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `F24B8B4C9EC58C203D4F2D3DF379B21AD5793C5ABFCE5BBD278F4152F0EE28D1`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `4E6D4DD7F56106C589B1F7BD414AF296B293096596EA8BDB2FC9532BC25D8930`

## Rollback

- Revert this task commit to restore editor-local selection action triggering and strict malformed JSON handling.
