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
- Malformed structured output now recovers a visible `reply` when possible and also recovers `edits` from JSON-looking code blocks so explicit edit requests still reach the draft replacement pipeline.
- Added parser tests for broken JSON payloads, unescaped quotes in `reply`, and screenshot-style malformed JSON edits.
- Updated the project map with the durable selection action behavior.

## Verification

- `npm run build`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test parse_cocreation_model_output --manifest-path src-tauri\Cargo.toml`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed.
- `cmd.exe` with Visual Studio `vcvars64.bat`, then `npm run tauri -- build`: passed.
- Updated local artifacts:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `F24B8B4C9EC58C203D4F2D3DF379B21AD5793C5ABFCE5BBD278F4152F0EE28D1`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `4E6D4DD7F56106C589B1F7BD414AF296B293096596EA8BDB2FC9532BC25D8930`
- Rebuilt after malformed JSON edit recovery update:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `01C9589D5167C7CE69B11CA6B01C674D723AFABD545ED8A13AFB7ACFDAA20A51`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `E35D8CA5EFC3ECE8704B9BD59E3EFD11FA1F436E054E2896560D8338967735F0`

## Rollback

- Revert this task commit to restore editor-local selection action triggering and strict malformed JSON handling.
