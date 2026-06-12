# 2026-06-12 Knowledge Library File Open Preview Edit Fix

## Issue

- In the knowledge library tab, Markdown files from the default knowledge base appeared in the file tree but failed to open in the editor.
- The editor title could switch to the requested knowledge file while the body still showed the previous works-library content.
- The UI surfaced `文件不在当前 Wridian 工作目录内。`.

## Root Cause

- The backend workspace file guard only added the default knowledge root to the allowed local roots when no work root existed.
- After the user selected a works library, files under the default knowledge root still appeared in `knowledgeFiles` but were rejected by `wridian_open_file`, `wridian_preview_file`, `wridian_preview_asset`, `wridian_save_file`, and folder/node operations.

## Fix

- `allowed_work_roots` now always includes the resolved knowledge root, whether it is user-configured or the default Wridian knowledge base.
- Root registration de-duplicates canonical paths to keep overlapping configured/default roots stable.
- Common file handling remains shared with the works library: editable Markdown/text/docx, text preview formats, and asset preview formats continue to use the same safety guard.

## Verification

- Added regression coverage for a selected works root plus default knowledge root:
  - knowledge Markdown is editable/readable;
  - knowledge text file is preview-readable;
  - knowledge PDF is accepted as a supported preview file;
  - knowledge folders are accepted for file-tree operations.
- `.\scripts\cargo-msvc.ps1 test selected_work_root_does_not_block_default_knowledge_files --manifest-path .\src-tauri\Cargo.toml`: passed.
- `.\scripts\cargo-msvc.ps1 test workspace_tree_displays_common_files_and_edits_word_notes --manifest-path .\src-tauri\Cargo.toml`: passed.
- `.\scripts\cargo-msvc.ps1 test --manifest-path .\src-tauri\Cargo.toml`: 106 passed.
- `npm run build`: passed.

## Rollback

- Revert the workspace root guard change and the regression test in `src-tauri/src/workspace.rs`.
