# Gemini OAuth client 配置入口修正

## 背景

截图红框显示 Gemini OAuth 要求先配置本机环境变量 `WRIDIAN_GOOGLE_OAUTH_CLIENT_ID` 和 `WRIDIAN_GOOGLE_OAUTH_CLIENT_SECRET`。这会让安装版用户误以为必须修改系统环境。

## 变更

- Gemini OAuth provider 不展示连接参数输入项，用户只需要点击浏览器 OAuth 登录。
- OAuth 登录命令只接收模型列表，后端默认使用 Gemini CLI 同款公共桌面 OAuth client。
- `WRIDIAN_GOOGLE_OAUTH_CLIENT_ID` / `WRIDIAN_GOOGLE_OAUTH_CLIENT_SECRET` 仅保留为高级环境变量覆盖项，不写入 provider，不展示给普通用户。
- 接入详情折叠按钮改名为“查看/收起接入详情”，避免与可填写的第三方 provider 连接参数混淆。

## 回退

回退本轮修改可恢复到显式连接参数方案；不影响已保存到 Windows Credential Manager 的 OAuth token，但会再次把 Gemini OAuth client 暴露给普通用户填写。
