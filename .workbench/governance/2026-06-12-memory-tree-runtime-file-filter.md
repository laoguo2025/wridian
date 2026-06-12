# 2026-06-12 Memory Tree Runtime File Filter

## Issue

- Knowledge fold files, knowledge health reports, compressed files, and other generated runtime files appeared as visible memory tree leaves.
- This made operational artifacts look like user-authored memory.

## Root Cause

- The memory tree backend appended every Markdown file from the knowledge library into the `knowledge` branch as `knowledge-card`.
- Project memory folders exposed `compressed.md` through the same visible leaf traversal used for ordinary Markdown leaves.

## Fix

- Removed knowledge-library Markdown synchronization from the creative memory tree response.
- Kept the `knowledge` branch as a rule/call mechanism branch; knowledge cards enter context through explicit `@` selection or knowledge graph flows, not as memory leaves.
- Added a visible-leaf filter for internal runtime files:
  - `compressed.md`
  - `compact-summary.md`
  - `hot.md`
  - `knowledge-fold-*`
  - `知识库体检-*`
- Project `compressed.md` remains readable by internal project-continuity context, but is no longer returned to the visible memory tree leaf view.

## Verification

- `.\scripts\cargo-msvc.ps1 test memory_tree_does_not_sync_knowledge_files_as_leaves --manifest-path .\src-tauri\Cargo.toml`: passed.
- `.\scripts\cargo-msvc.ps1 test memory_tree_hides_project_compressed_file_from_leaf_view --manifest-path .\src-tauri\Cargo.toml`: passed.
- `.\scripts\cargo-msvc.ps1 test project_continuity_memory_reads_only_project_memory_tree_files --manifest-path .\src-tauri\Cargo.toml`: passed.

## Rollback

- Revert the memory-tree backend filter and restore the previous knowledge-card sync behavior in `src-tauri/src/memory.rs`.
