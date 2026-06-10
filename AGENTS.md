# Wridian 项目规则

- 以后打包测试版，指的就是无需安装的 exe，而且统一保存在 `D:\Coding\Wridian\release` 里。

## GitHub 推送固定流程

- 默认只使用标准 Git CLI 推送，不临场切换 GitHub API：`git fetch origin master`、整理到远端队尾后、`git push origin HEAD:master`。
- 推送前必须确认远端队尾：`gh api repos/laoguo2025/wridian/branches/master --jq .commit.sha`，再 `git fetch origin master`。如果本地 `origin/master` 落后于远端，先修正本地跟踪分支或从 `origin/master` 新建线性同步分支。
- 如果本地历史因为历史 API 同步而与远端分叉，不做 merge commit，不 force push；从 `origin/master` 新建 `codex/sync-...` 分支，只 cherry-pick 需要发布的本地提交，然后普通 push 到 `master`。
- 如果 `git fetch` 或 `git push` 失败，先诊断并修复标准 Git 通道：`Test-NetConnection github.com -Port 443`、`gh auth status`、`git remote -v`、`git config --get-regexp "credential|http"`。不得未经诊断直接改用 GitHub API。
- GitHub API 只作为最后兜底，并且必须先向用户说明原因；兜底时只能使用固定脚本 `node .workbench/tools/github-tree-sync.mjs laoguo2025/wridian master`，禁止临时手写 API 推送脚本。
- 正式安装包需要入库时只纳入 `release/Wridian-x.y.z-x64-setup.exe`；测试版 exe 保留本地 `release/`，不推仓库。不要为了新版本发布改写旧版本安装包。
