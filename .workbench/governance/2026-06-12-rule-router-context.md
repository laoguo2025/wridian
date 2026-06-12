# P1 Rule Router Context

## 目标

解决 P1：让作品库和知识库根目录中的 `WRIDIAN.md` / `AGENT.md` / `AGENTS.md` / `index.md` / `hot.md` 进入对话上下文，并保持它们与创作记忆树、知识卡和技能规则分槽。

## 变更

- 新增后端 `rule_router` 模块，分别读取当前作品库根目录和知识库根目录的规则、索引与 hot 文件。
- 在对话 prompt 中新增“规则路由”独立槽位，位于文件树之后、项目记忆之前。
- 上下文命中状态新增 `rule-router`，前端显示为“规则路由”。
- 更新项目地图中的上下文槽位长期口径。

## 边界

- 只读取库根目录下固定文件名，不递归扫描。
- 跳过符号链接、非普通文件和超过 128 KiB 的规则文件。
- 只向模型发送库类型、文件名和内容，不发送本机绝对路径。
- 规则路由不写入创作记忆树，也不占用技能规则槽位。

## 回退

回退本轮提交即可移除 `rule_router` 模块、prompt 槽位、前端标签和项目地图口径。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml rule_router`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation`
- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`
