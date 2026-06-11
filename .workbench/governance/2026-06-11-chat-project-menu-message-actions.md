# Chat Project Menu And Message Actions

## Scope

- Fixed the project selector so an empty project list does not render a blank dropdown shadow.
- Moved the project selector dropdown through a body portal so it opens to the left of the trigger even inside the blurred chat panel.
- Replaced message action text with icon-only buttons and kept tooltips for context, edit or retry, and copy.
- Kept message time only at timeline separators, not inside bubbles.
- Added `mimo-v2.5` to Xiaomi MiMo default model presets.

## Verification

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`
- `git diff --check`
- Playwright browser check with mocked Tauri commands:
  - no project click leaves no `.chat-project-menu`
  - project menu is portaled to `body` and opens left of the trigger
  - user actions are three SVG icon buttons with Chinese labels
  - context popover shows only loaded Chinese context items
  - copy hint shows `复制成功`
  - inline edit persists edited message text
  - assistant actions have no `分叉`

## Rollback

Revert the UI changes in `src/chat/ChatPanel.tsx`, `src/App.css`, `src/icons.tsx`, the edit persistence helper in `src/chat/chatManager.ts`, and the Xiaomi preset addition in `src/settings/providerCatalog.ts`.
