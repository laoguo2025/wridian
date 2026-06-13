# 2026-06-13 literal replace precision

## Scope

- Fix imprecise chat-driven draft edits for explicit literal replacement commands such as `把牛魔王都改成猪八戒`.

## Root Cause

- Wridian previously sent all `改成` and `替换` requests to the model first.
- The model could interpret a literal rename as a story consistency rewrite, producing edits for related terms such as roles or relationships.
- Repeated target words were also treated as ambiguous unless a precise source range existed, causing misleading safety messages.

## External Reference

- `claudian` wraps active editor context in `<editor_selection path="...">` and `<editor_cursor path="...">`, then uses inline diff/apply around that fixed editor context.
- Its inline edit modal keeps the editor/view captured at command invocation time, avoiding loose active-file inference during apply.
- Local `obsidian-copilot` checkout was incomplete in `.workbench/runtime/external-repos`, so this pass did not use it as current-source evidence.

## Changes

- Added a local fast path for explicit literal replacement instructions.
- The fast path parses commands like `把 A 都改成 B`, scans the current draft or selected text, and creates one pending edit per exact match.
- Each generated edit carries a `sourceRange`, so repeated occurrences can be rendered and confirmed individually without ambiguity.
- The fast path bypasses the model, preventing semantic expansion beyond the user-specified literal terms.
- Updated unsafe-location messages so they describe imprecise model suggestions or changed text instead of implying the wrong file is always open.
- Updated the project map with the durable rule.

## Verification

- Manual parser smoke: `把牛魔王都改成猪八戒`, `请把「牛魔王」全部替换为「猪八戒」`, and `将 牛魔王 统一 改为 猪八戒` all parse to `牛魔王 -> 猪八戒`.
- Manual range smoke: repeated `牛魔王` occurrences produce distinct ranges.
- `npm run build`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`: passed.
- `cmd.exe` with Visual Studio `vcvars64.bat`, then `npm run tauri -- build`: passed.
- Updated local artifacts:
  - `release\Wridian-0.0.8-x64-setup.exe`, SHA256 `9EF7C29974067278DFA18B4932CB30CDE8B6649F97F9B349FC52B0C9258F21F7`
  - `release\Wridian-0.0.8-test.exe`, SHA256 `3D8CC0469AD3E710852F9229F3FCA2B513ACB37221151F69CA44C37815AFD107`

## Rollback

- Revert this task commit to return to model-first edit generation for all chat replacement requests.
