# 2026-06-12 Knowledge Health Cache Visibility

## Scope

- Hide system cache artifacts from the left knowledge file tree.
- Preserve the latest knowledge health result panel while the user opens a report or closes/reopens the knowledge graph drawer.

## Changes

- Knowledge file tree now skips `hot.md` and `00知识库治理/folds`.
- Knowledge health reports remain visible in `00知识库治理`.
- The latest health result is stored in `App` and passed into `KnowledgeGraphDrawer`, so closing the drawer or opening a report no longer clears the result panel.

## Verification

- `npm run build`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace --lib`: passed, 7 tests.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_graph --lib`: passed, 5 tests.
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`: passed, 105 tests.

## Rollback

- Revert `src-tauri/src/workspace.rs`, `src/App.tsx`, and `src/knowledge/KnowledgeGraphDrawer.tsx`.
