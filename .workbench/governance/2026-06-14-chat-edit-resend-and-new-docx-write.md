# 2026-06-14 chat edit resend and new DOCX write

## Scope

- Fix chat user-message edit submit so it resends the edited prompt instead of only changing stored text.
- Fix chat-driven creation of a new `.docx` file with generated content.
- Rebuild and install `release/Wridian-0.0.9-x64-setup.exe`.

## Findings

- User-message edit submit previously called the same message text update path used by assistant messages, so it changed the bubble content but did not create a new model request.
- New `.docx` writes used the existing-DOCX rewrite path. For a new file, that path attempted to read the target first and failed with `os error 2`.

## Changes

- User-message edit submit now truncates the chat branch at the edited user message, restores its context pills, and sends the edited text as a new prompt.
- `useChatManager.sendPrompt` builds the next message queue from `messagesRef.current`, so branch truncation is respected before the resend.
- New `.docx` writes now create a minimal readable DOCX package directly. Existing `.docx` writes still use the editable-DOCX rewrite path.
- E2E smoke now covers edited user-message resend and rejects stale assistant replies after submit.

## Verification

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace --lib`
- `npm run tauri -- build`
- Built exe smoke: `scripts\e2e-launch.ps1 -DebugPort 9444 -StopExisting`; `node scripts\e2e-smoke.mjs`
- Installed exe smoke: `scripts\e2e-launch.ps1 -ExePath %LOCALAPPDATA%\Wridian\Wridian.exe -DataDir .workbench\runtime\installed-e2e-data -DebugPort 9555 -StopExisting`; `node scripts\e2e-smoke.mjs`
- Real model edit-resend test with installed exe, isolated data dir, copied real model config, no mock queue:
  - First prompt generated one real assistant reply.
  - Edited user prompt submit removed the stale branch and generated one new assistant reply.
  - Audit log recorded two real provider responses at `2026-06-14T15:38:42+08:00` and `2026-06-14T15:38:50+08:00`.

## Package

- Installer: `release/Wridian-0.0.9-x64-setup.exe`
- SHA256: `CD8AC1FA88EABDC454E279F51EF42A37B4803C76DD04785BFAEBDAD253C5A36F`
- Installed exe: `%LOCALAPPDATA%\Wridian\Wridian.exe`
- Installed exe SHA256: `756D2769DCFA56E41570125485A5E78F0345D588552264B107CA638F10ACD4FD`
