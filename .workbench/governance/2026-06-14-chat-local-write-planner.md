# 2026-06-14 Chat Local Write Planner

## Problem

User request reproduced from the installed app:

`根据第1集剧情，续写第2集，在作品库新建个第2集文档`

The model returned a fake completion or summary without `fileOperations`, so Wridian showed the missing-file-operation blocker and did not create the file.

## Reference Check

- `YishenTu/claudian`: file changes are surfaced as provider tool events (`tool_use` / `tool_result`) and normalized for the chat UI; a file change is only considered done after the runtime reports the file-change result.
- `logancyang/obsidian-copilot`: agent mode binds tools, executes each call through the host `ToolManager.callTool`, then feeds tool outputs back into the LLM flow. Tool execution is host-owned, not inferred from assistant prose.

## Change

- For explicit chat requests that ask to create a new document, Wridian now plans the local `writeFile` target before trying to repair JSON output.
- The model is asked only for the new document body; Wridian then executes the constrained local `writeFile` operation.
- Fake assistant claims, summary replies, edits, and memory drafts from the first response are not written as file content.
- Added local planner audit at `.wridian/cocreation-local-write-planner.jsonl`, recording stage, library, relative path, user input, reply kind, and failure reason without storing full generated manuscript text.

## Verification

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml e2e --lib`
- `node --check scripts\e2e-smoke.mjs`
- MSVC env: `npm run tauri -- build`
- `powershell -ExecutionPolicy Bypass -File scripts\e2e-launch.ps1 -DebugPort 9222 -StopExisting; node scripts\e2e-smoke.mjs`

Real exe E2E verified:

- works library chat create/rename/trash
- knowledge library chat create/rename/trash
- selected text added to chat and sent
- Markdown table rendered in chat bubble
- exact episode request creates `测试/第2集.md`
- generated file content is the episode body, not the assistant summary
- current opened draft is unchanged and no inline diff is created for the new-file request

Release installer copied to `release/Wridian-0.0.9-x64-setup.exe`.
SHA256: `BBD5ED8D15047F4BFDF583236CC767506F684B94F028386D281C5C3C5937BDBC`
