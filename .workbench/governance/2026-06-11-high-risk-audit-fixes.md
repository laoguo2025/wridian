# 2026-06-11 High Risk Audit Fixes

## Scope

- Fix draft loss and stale response races when switching files.
- Fix duplicate prompt submission races in chat.
- Block symlink, junction and reparse-point traversal during workspace copy and recursive Markdown scans.

## Change Rationale

- File switching previously replaced editor state before a pending autosave could complete.
- Chat submission relied on React state for same-tick duplicate prevention.
- Recursive copy, knowledge graph, relevant notes and memory tree scans used `is_dir()` on child paths, which can follow directory links outside the selected root.

## Rollback

- Revert the frontend changes in `src/App.tsx` and `src/chat/chatManager.ts`.
- Revert `src-tauri/src/path_safety.rs` plus its uses in workspace, graph, projects and memory modules.

## Verification

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml path_safety`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace`
