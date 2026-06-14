# 2026-06-14 hide successful file operation blocks

## Scope

- Hide successful file-operation result cards from chat bubbles.
- Keep failed file-operation cards visible so users can still see when an operation did not execute.

## Findings

- The assistant reply already tells the user that the document was written, and the file tree/editor reflect the result.
- Rendering a separate successful `写入文件 / 已执行` card adds internal tool noise to the conversation.

## Changes

- `FileOperationBlocks` now filters out successful operations before rendering.
- The block returns `null` when all operations succeeded.
- E2E smoke now asserts successful file-operation blocks are hidden.

## Verification

- `npm run build`
- MSVC environment `npm run tauri -- build`
- Installed `release/Wridian-0.0.9-x64-setup.exe` silently.
- Installed exe smoke passed with `scripts\e2e-launch.ps1 -ExePath %LOCALAPPDATA%\Wridian\Wridian.exe -DataDir .workbench\runtime\installed-e2e-data -DebugPort 9555 -StopExisting` and `node scripts\e2e-smoke.mjs`.

## Package

- Installer: `release/Wridian-0.0.9-x64-setup.exe`
- SHA256: `933F7FE4C93552F24502C63B0CFEE3157416FA8D661538FB4E3B88494F736751`
- Installed exe SHA256: `25999E11305873DB3E658BB439C6FA7D1A6248A326CD7EB89E09317DF73BB077`
