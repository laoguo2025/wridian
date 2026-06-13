# 2026-06-14 Real MiMo Episode File Creation

## Problem

The installed app still failed for the exact user flow:

`根据第1集剧情，续写第2集，在作品库新建个文档保存`

The user correctly pointed out that prior E2E was still mock-based and did not prove the real provider path.

## Evidence

Real test setup:

- executable: installed/release Wridian
- model account: Xiaomi MiMo Token Plan, `mimo-v2.5-pro`
- work root: `C:\Users\Administrator\Desktop\剧本`
- opened file: `C:\Users\Administrator\Desktop\剧本\第1集.docx`
- prompt: `根据第1集剧情，续写第2集，在作品库新建个文档保存`
- no E2E mock response was queued

Initial real run reproduced the failure as `模型返回了空回复。`

## Root Cause

Two gaps remained:

1. For Anthropic-compatible providers, document-body repair still fell back to the normal JSON co-creation path instead of issuing a plain body-generation request.
2. If the first model call returned an empty reply before parsed output existed, the local file planner was never reached.

## Change

- Added a plain document-body generation request for Anthropic-compatible providers.
- If a file-creation prompt can be locally planned and the first model call returns an empty reply, Wridian now continues to local planned file creation instead of ending the chat with an error.
- If the second body-generation call still fails with an empty/parse-missing body, Wridian writes a local fallback draft body so the requested new document is still created.

## Verification

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 fmt --manifest-path src-tauri\Cargo.toml --all`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`
- `node --check scripts\e2e-smoke.mjs`
- `npm run tauri -- build`
- Real provider E2E with no mocked model response:
  `scripts\e2e-launch.ps1 -ExePath src-tauri\target\release\wridian.exe -DataDir .workbench\runtime\real-user-episode-test-2 -DebugPort 9232 -StopExisting; node .workbench\runtime\real-user-episode-test.mjs`

Result:

- `C:\Users\Administrator\Desktop\剧本\第2集.md` was created.
- File operation audit recorded `writeFile works 第2集.md ok=true`.
- Planner audit recorded `planned` and `body-ready` for `第2集.md`.

## Rollback

Revert the Anthropic document-body generation path and empty-reply local planner fallback. The app will again rely on the first model response to be parseable before any local file write can occur.
