# 2026-06-14 Clean Document Body Repair Context

## Problem

A fresh user message still failed:

`根据第1集剧情，续写第2集，在作品库新建个文档保存`

The file planner correctly inferred `works / 第2集.md`, but no file operation was applied.

## Evidence

Installed-app planner audit showed:

- `planned`: `library=works`, `path=第2集.md`
- `rejected-body`: model did not return standalone document body

The latest chat session stored the same new user message, proving the failure was not merely an old visible chat bubble.

## Root Cause

The second-stage "generate document body" repair prompt reused the full co-creation prompt. That prompt included active-context and compressed-memory slots, which could contain stale prior failure summaries such as:

- last user intent
- last judgment
- "model did not return executable file operations"

For a repair request, this polluted the model input and made it more likely to repeat a summary/claim instead of producing only the new document body.

## Change

Document-body repair now builds a clean, narrow prompt:

- target file path
- current user request
- current opened draft content
- mentioned file contents when present
- compact file tree only for collision awareness

It intentionally excludes active chat context, compressed memory, project memory, and old repair failure judgments.

## Verification

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 fmt --manifest-path src-tauri\Cargo.toml --all`
- `node --check scripts\e2e-smoke.mjs`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml e2e --lib`
- `npm run build`
- fresh-data E2E against release exe:
  `scripts\e2e-launch.ps1 -DataDir .workbench\runtime\e2e-fresh-episode -DebugPort 9223 -StopExisting; node scripts\e2e-smoke.mjs`
- `npm run tauri -- build`
- fresh-data E2E against newly packaged release exe:
  `scripts\e2e-launch.ps1 -ExePath src-tauri\target\release\wridian.exe -DataDir .workbench\runtime\e2e-fresh-packaged -DebugPort 9224 -StopExisting; node scripts\e2e-smoke.mjs`

The E2E now uses the user's fresh prompt form and creates `第2集.md` from a clean fixture.

## Rollback

Revert the clean document-body prompt and E2E prompt update. The previous full-context repair behavior will return, including the known risk that stale chat failures bias document-body generation.
