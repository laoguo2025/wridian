# 2026-06-14 Chat Direct Content Write Order

## Problem

User reported that creating an empty file from chat worked, but combining file creation with content writing failed. The concrete failing flow was:

- create `早上好.md` from chat: success, empty file
- ask chat to write content while creating a new document, or continue `第1集.docx` into `第2集`: failed before a real file write, sometimes with an empty model reply message

## Root Cause

The chat co-creation path still reached model settings / model response handling before a deterministic local write could run for direct content requests. If the provider returned an empty response, the request failed before any local `writeFile` operation was planned or applied.

There was also an overly broad parent-folder parser that could treat natural phrasing like "作品库里新建..." as an explicit folder name.

## Change

- Direct requests with explicit content markers such as "内容写..." now run through a local write path before model settings or model calls.
- Direct content is only accepted when the extracted body is not an instruction-like sentence.
- Continuation requests such as "根据第1集剧情，续写第2集..." do not use the direct-content fast path; they still ask the model for body text, then Wridian performs the file write locally.
- Parent-folder parsing now only accepts explicit folder phrasing such as "作品库的 X 里".
- E2E now exposes chat pending state so prompts wait for real completion instead of timing guesses.

## Verification

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 fmt --manifest-path src-tauri\Cargo.toml --all`
- `node --check scripts\e2e-smoke.mjs`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml e2e --lib`
- `npm run build`
- `npm run tauri -- build`
- `powershell -ExecutionPolicy Bypass -File scripts\e2e-launch.ps1 -DebugPort 9222 -StopExisting; node scripts\e2e-smoke.mjs`

The packaged release exe E2E covered conversation-driven file tree operations in works and knowledge libraries, selected-text insertion into chat, direct create-with-content, and the exact docx `第1集` to `第2集` creation flow.

## Rollback

Revert the local direct write helpers and the E2E pending-state additions from this change. The previous model-first behavior will return, including the known empty-model-response failure mode for direct create-with-content prompts.
