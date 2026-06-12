# 2026-06-13 cocreation plain text response fallback

## Scope

- Fix chat send failures that show `对话结果不是有效 JSON` when the selected model returns ordinary prose instead of Wridian's structured JSON object.
- Applies both with and without selected-text context pills.

## Root Cause

- `parse_cocreation_model_output` required every model reply to parse as JSON or contain an extractable JSON block.
- Some OpenAI-compatible providers/models can ignore `response_format` or system instructions and return plain text. In that case Wridian treated a valid assistant answer as a transport failure.

## Changes

- Plain text model output now becomes a normal assistant `reply` with empty `edits`, `fileOperations`, and `memories`.
- If output contains an extractable JSON payload but that payload is malformed, Wridian still reports the JSON error instead of silently swallowing a broken structured edit/file operation response.

## Verification

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test parse_cocreation_model_output --manifest-path src-tauri\Cargo.toml`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed.
- `npm run build`: passed.
- `cmd.exe` with Visual Studio `vcvars64.bat`, then `npm run tauri -- build`: passed.
- Updated local artifacts:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `84A46AF72D89F9292A6765A73A4193BAA0F2AE6B80DFDA77C9DABC0D76E7F169`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `6F974B448FD0537C64FC84C7A9369DC33A8E18DCF90EEF42BFEA7E97C2D8651E`

## Rollback

- Revert this task commit to restore strict JSON-only cocreation output parsing.
