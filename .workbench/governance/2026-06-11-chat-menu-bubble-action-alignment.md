# Chat Menu And Bubble Action Alignment

## Scope

- Align the project dropdown top edge with the `普通聊天` trigger while keeping it on the left side.
- Move message action icons outside the bubble, anchored at the bubble bottom-right corner.
- Hide the context icon for assistant messages.
- Render context hit details directly in the popover without a nested `已引用上下文` disclosure.
- Close context and modify states when the user clicks elsewhere, and modify message text directly inside the bubble without a textarea.
- Use a Doubao-style modify state with cancel, inline editable text, and submit controls.
- In modify mode, replace the message bubble with a rounded edit control instead of nesting an editor inside the bubble; keep the submit button on Wridian accent color and constrain the control inside the chat panel.

## Verification

- `npm run build`
- Playwright mocked Tauri browser check:
  - dropdown top matches trigger top and opens left of the trigger
  - user action icons are outside the user bubble bottom-right edge
  - assistant action icons are outside the assistant bubble bottom-right edge and only show retry/copy
  - context popover contains direct hit items without `details`, `summary`, or `已引用上下文`
  - context popover appears below the context icon; copy hint appears below the copy icon
- outside click closes context and saves inline bubble modification
- modification mode has cancel and submit controls, uses no textarea, and exposes the action as `修改`
- modification mode has no outer `.chat-message-bubble`, stays within the chat panel, and uses `--accent` rather than the blue focus color for submit
- follow-up verification after the Doubao comparison: modification mode now expands as a full-row editor strip instead of a short right-aligned bubble-sized control; mocked browser measurement showed shell width ratio `0.987`, no chat panel overflow, no outer bubble, no textarea, transparent edit shell, and submit background `rgb(220, 125, 87)`
- follow-up refinement: edit submit uses no glow/shadow, and only the latest user message in the current conversation renders the modify action; older user messages keep context/copy only

## Rollback

Revert `src/chat/ChatPanel.tsx` and `src/App.css` to the previous message action and project menu layout.
