# 2026-06-13 edit intent auto apply

## Scope

- Fix the experience where user-requested rewrite/replace operations only showed inline candidates and required extra confirmation.
- Keep normal chat and suggestion requests from changing the draft automatically.

## Root Cause

- Wridian still used the older safety model where all model `edits` became pending inline diffs.
- The prompt also told the model that edits would only take effect after user confirmation, which encouraged "choose one" style replies.

## Changes

- Frontend now auto-applies safely locatable edits only when the user prompt has explicit edit intent such as rewrite, modify, replace, organize current draft text, polish, delete, or batch operation.
- Normal chat, explanation, suggestion, and comparison requests keep edits non-auto-applied. Broad words such as organize/optimize/batch only auto-apply when the prompt also names a draft/content target.
- Auto-apply uses the existing replace guard: unique/range-safe matches are written to the editor and retain one undo snapshot; unsafe edits remain pending.
- Prompt text now tells the model to provide a single concrete edit result only for explicit edit requests, and to keep edits empty for ordinary discussion.
- Project map updated so the durable behavior says explicit edit requests can auto-write safe edits, while unsafe edits remain pending inline diffs.

## Verification

- `npm run build`: passed after final intent-boundary tightening.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed after final intent-boundary tightening.
- `cmd.exe` with Visual Studio `vcvars64.bat`, then `npm run tauri -- build`: passed.
- Updated local artifacts:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `083084671EA943248A5843B5878CB4240E7CAC603BA19913ACD8554A1CFF5A0B`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `0A7CF6CB186F1F5569060DD803F1C82BA751657B7AEA245FC5B4C9125CB805F0`

## Rollback

- Revert this task commit to restore pending-only edit behavior.
