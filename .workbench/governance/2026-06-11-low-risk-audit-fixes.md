# Low Risk Audit Fixes

## 原因

继续收口低风险技术债：中风险修复后仍有旧 custom API 命名残留、前端类型中保留已下线配置结构、供应商目录存在未消费字段，以及未知协议展示仍可能误导为 OpenAI-compatible。

## 变更

- 删除前端已下线的 `CustomApiSettingsStatus` 类型。
- 将模型连接测试响应从旧 `TestCustomApiResponse` 命名改为 `TestModelProviderResponse`。
- 移除 provider catalog 中当前未被 UI、保存链路或后端消费的 `iconKey` 和 `defaultRoleModels` 字段。
- `protocolLabel` 对未知协议显示“未知协议”，不再 fallback 为 OpenAI-compatible。

## 验证

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml`（48 passed）

## 回退

如需回退，可恢复本次删除的类型/字段和旧响应命名；但会重新引入已下线接口命名、死字段维护成本和未知协议展示误导。
