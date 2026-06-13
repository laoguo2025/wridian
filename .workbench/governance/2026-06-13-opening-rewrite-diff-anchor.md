# 2026-06-13 Opening Rewrite Diff Anchor

## 背景

截图中用户要求重写开场段，但模型返回的 edit target 没有在当前正文中精确命中，导致右侧提示“没有安全定位”，正文区只显示“需重新定位”，没有红绿内联 diff。

## 变更

- 保留默认严格定位：模型 target 能唯一命中正文时仍按原 target 定位。
- 当用户明确要求重写、改写、润色或修改“开头/开场/开篇/第一段/前几段”等开场范围，且模型 target 无法唯一命中时，前端将该 edit 兜底锚定到当前正文开头的真实连续段落。
- 如果本轮带选区且用户是改写意图，则优先锚定选区真实范围。
- 兜底只替换 edit 的 target 和 sourceRange，不自动写入正文；仍需要用户在正文红绿 diff 中确认。

## 验证

- `npm run build` 通过。

## 回退

回退 `src/chat/chatManager.ts` 中 `createPendingDraftEdits` 的输入参数调整和 `createOpeningRewriteFallback` 相关辅助函数。
