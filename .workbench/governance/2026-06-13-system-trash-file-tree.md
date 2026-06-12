# 2026-06-13 文件树移到系统回收站

## 目标

- 文件树右键“移到回收站”改为用户本机系统回收站。
- 模型 `fileOperations.trash` 保持同一删除语义。
- 保持当前作品库和知识库路径边界校验，不允许操作库外文件。

## 变更

- 新增 `trash` Rust 依赖，运行时通过系统回收站 API 移动文件或文件夹。
- 后端 `wridian_trash_work_node` 和 `apply_workspace_trash_node` 不再创建或写入库内 `.wridian-trash`。
- 对话 prompt 中的 `trash` 协议说明改为系统回收站。
- 安装包内置创作技能中关于错误产物回滚的说明同步为系统回收站。
- 项目地图和 README 同步更新。

## 验证

- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace_trash` 通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml` 通过，119 个测试全部通过。
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml` 通过，覆盖非测试构建中的 `trash::delete` 路径。
- `npm run build` 通过。

## 回退

- 移除 `trash` 依赖，并恢复 `wridian_trash_work_node` / `apply_workspace_trash_node` 到 `.wridian-trash` rename 实现。
