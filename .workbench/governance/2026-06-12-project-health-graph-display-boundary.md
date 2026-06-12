# 2026-06-12 Project Health Graph Display Boundary

## Scope

- Treat `project.md` as an internal project memory file, not a visible memory-tree leaf.
- Generate knowledge health reports with date-time filenames instead of day-level filenames.
- Fix knowledge graph hover display so it uses library-relative paths and does not show file body content.

## Fix

- Added `project.md` to the memory-tree visible-leaf runtime-file filter.
- Changed knowledge health report filenames from `知识库体检-YYYY-MM-DD.md` to timestamped `知识库体检-YYYYMMDDTHHMMSS*.md`.
- Added `relativePath` to knowledge graph nodes from the backend.
- Updated graph hover preview to show `relativePath`/group/label only plus metadata; it no longer opens files or displays text content on hover.

## Verification

- `.\scripts\cargo-msvc.ps1 test memory_tree_hides_project_core_runtime_files_from_leaf_view --manifest-path .\src-tauri\Cargo.toml`: passed.
- `.\scripts\cargo-msvc.ps1 test health_report_timestamp_uses_date_time_filename --manifest-path .\src-tauri\Cargo.toml`: passed.
- `.\scripts\cargo-msvc.ps1 test graph_keeps_response_shape_when_built_from_index --manifest-path .\src-tauri\Cargo.toml`: passed.
- `python -m py_compile .\src-tauri\resources\knowledge-health\scripts\health_check_knowledge_base.py`: passed.
- `.\scripts\cargo-msvc.ps1 test --manifest-path .\src-tauri\Cargo.toml`: 106 passed.
- `npm run build`: passed.

## Rollback

- Revert the memory filter, health report timestamp naming, and graph hover preview changes in the task commit.
