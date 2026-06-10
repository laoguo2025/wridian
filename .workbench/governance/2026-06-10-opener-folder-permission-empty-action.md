# Opener Folder Permission And Empty Action

## Root Cause

- The toolbar folder button uses `@tauri-apps/plugin-opener` `openPath` when a library root exists.
- The desktop capability only granted `opener:default`, which allows URLs and reveal-in-folder behavior, but not direct local path opening.
- Installed builds therefore rejected `openPath` as not allowed, and the frontend showed the misleading desktop fallback message.

## Change

- Added `opener:allow-open-path` to the default desktop capability.
- Removed the file tree empty-state `选择作品库文件夹 / 选择知识库文件夹` button. Folder selection remains available from the toolbar folder icon when no root is configured.

## Validation

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`

## Rollback

- Remove `opener:allow-open-path` from `src-tauri/capabilities/default.json`.
- Restore the removed empty-state button and `.library-empty-action` CSS if the product wants a large empty-state selector again.
