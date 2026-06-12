# 2026-06-13 低优先级审查修复记录

## 范围

- 将生产 CSP 中的开发 localhost 连接源移入 `devCsp`。
- 同步 README 和项目地图中卸载清理描述，明确只清 Wridian 运行产物，不直接删除整个知识库根目录。
- 清理内置技能和知识库体检资源中的固定桌面知识库路径，改为参数或 `WRIDIAN_KNOWLEDGE_ROOT`。
- 删除前端已无后端命令对应的知识库缓存响应类型。
- 优化知识图谱布局初始化，避免按节点重复扫描同层节点。
- 增加 `.gitattributes` 约束 shell 脚本使用 LF，并修复知识卡体检 shell 脚本在 bash 下被 CRLF 破坏的问题。

## 验证

- `npx tsc --noEmit`
- `npm run build`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cmd.exe /d /s /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul && cargo test --manifest-path src-tauri/Cargo.toml'`
- `python -m json.tool src-tauri/tauri.conf.json`
- `python src-tauri/resources/knowledge-health/scripts/init_knowledge_base.py` 应报缺少 `--root` 或 `WRIDIAN_KNOWLEDGE_ROOT`。
- `python src-tauri/resources/knowledge-health/scripts/health_check_knowledge_base.py` 应报缺少 `--root` 或 `WRIDIAN_KNOWLEDGE_ROOT`。
- 使用 `.workbench/runtime` 临时知识库验证 Python 初始化、Python 体检和 shell 体检脚本可执行；临时目录已清理。

## 回退

- 如需回退本轮变更，回退对应提交即可；本轮未写入用户知识库、未删除用户文件、未 push。
