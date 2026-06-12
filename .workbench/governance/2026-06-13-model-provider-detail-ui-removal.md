# 2026-06-13 模型接入详情 UI 删除

## 目标

- 删除模型连接二级弹窗里的“查看接入详情”入口及展开详情 UI。
- 不影响模型 provider 的保存、测试、OAuth 登录和内部 catalog 元数据。

## 变更

- `ModelSettingsDialog.tsx` 删除 `showAdvanced` 状态、接入详情 JSX 和外链打开回调。
- `App.css` 删除 `.provider-catalog-details` 和 `.provider-advanced-toggle` 相关样式。
- 项目地图同步记录二级连接弹窗不展示接入详情展开区。

## 验证

- `rg "查看接入详情|收起接入详情|provider-catalog-details|provider-advanced-toggle|openUrl|openExternal|onExternal|showAdvanced" src ...` 确认源码中无入口、样式和外链打开回调残留。
- `npm run build` 通过。

## 回退

- 恢复本次提交即可重新显示接入详情展开区。
