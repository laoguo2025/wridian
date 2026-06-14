# 2026-06-14 deterministic bulk replace routing

## Scope

- Fix chat-driven bulk literal replacements that previously left many edits in `需重新定位`.
- Keep the behavior generic instead of hardcoding specific names.

## Findings

- For prompts like `把第1集里的角色名牛魔王，都改成猪八戒`, the app still depended on model-returned `edits.target`.
- When the model returned contextual targets such as `牛魔王（喘着粗气）` or longer sentence fragments, many replacements could not be matched safely across the full draft.
- The frontend already had repeated-literal expansion, but that path only worked when the model returned a clean literal target.

## Changes

- Added a deterministic literal bulk-replace planner in `src/chat/chatManager.ts`.
- When the user intent clearly matches `把/将 A 都/全部/统一 改成|改为|替换成|替换为 B`, the app now:
  - parses literal `A` and `B` from the user message,
  - scans the current draft or the selected range directly,
  - generates one pending edit per exact occurrence with precise `sourceRange`.
- If deterministic planning succeeds, the app no longer depends on model `edits` for this class of request.

## Verification

- `npm run build`
- MSVC environment `npm run tauri -- build`
- Built exe smoke passed with the updated scenario:
  - model returns `edits: []`
  - prompt is `把第1集里的角色名牛魔王，都改成猪八戒`
  - expected result is multiple exact inline diffs and no `需重新定位`
- Installed exe smoke also passed after reinstalling the rebuilt `0.0.9` package.

## Package

- Installer: `release/Wridian-0.0.9-x64-setup.exe`
- SHA256: `C2447C6A8B18532F28ECE3038F63D6B872F4C0E094192A15CA2081E187E5174D`
- Installed exe SHA256: `F1B98E682661ED4AEF81EBCCE26D694702484351956EF8173DDDCD23AAC767F8`
