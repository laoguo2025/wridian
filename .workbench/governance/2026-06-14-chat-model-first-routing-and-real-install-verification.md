# 2026-06-14 chat model-first routing and real-install verification
- Removed the frontend pre-model literal-replace shortcut from chat sending.
- Kept local edit execution limited to model-returned edits, plus safe expansion for repeated replacements when the user says all/全部/都.
- Cleared stale prompt selection state after normal sends so a prior selection does not contaminate the next message.
- Expanded E2E coverage with two real scenarios: model-free semantic edit and ordered file-write flow.
- Updated the E2E fixture to include repeated 牛魔王 occurrences for the exact user scenario.
- Verified the fresh 0.0.9 installer package and the installed Wridian.exe both pass the clean E2E smoke on separate data dirs.
