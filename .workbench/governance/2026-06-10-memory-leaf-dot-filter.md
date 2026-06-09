# 2026-06-10 Memory Leaf Dot Filter

## 背景

用户指出记忆树上的暖橙色叶子点太大，并且当前显示的 `legacy-*.md` 圆点并不是真正叶子，而是旧主标签/伙伴文件迁移出来的内容。主标签内容和叶子形态尚未确定前，不应显示这些错误叶子点。

## 变更

- 记忆树叶子点尺寸从 10/12px 缩小到 7/9px。
- 前端叶子点列表过滤 `legacy-*.md`，保留真实叶子点功能。
- 后端停止把旧 `partner/user.md`、`partner/relationship.md`、`partner/partnermemory.md`、`global/AWARENESS.md` 复制到 leaves。
- 已有 legacy 文件不在本轮删除，避免误删用户可能已编辑的本地文件。
- 项目地图补充：旧迁移主文件不能显示为叶子点。

## 验证

- `cargo test --manifest-path src-tauri\Cargo.toml --lib` 通过，10 个测试。
- `cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- `npm run build` 通过。

## 回退

- 圆点尺寸集中在 `src/App.css` 的 `.memory-leaf-dot`。
- 叶子过滤集中在 `src/App.tsx` 的 `flattenMemoryLeaves`。
- 旧文件迁移边界集中在 `src-tauri/src/memory.rs` 的 `migrate_legacy_memory_files`。
