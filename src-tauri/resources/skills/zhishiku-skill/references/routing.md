# 路由规则

`zhishiku-skill` 是总控，但必须内置拆解、提炼、蒸馏三套子能力。独立子 skill 存在时优先调用；不存在时读取本 skill 的 embedded-skills 镜像继续执行。

## 路由表

| 用户说 | 动作 |
|---|---|
| 搭建知识库 / 初始化知识库 / 创建知识库 | 运行初始化脚本 |
| docx / pdf / epub / mobi / 超长文件 / 转成md或txt | 先归一化为 md/txt；过大则切分；再执行 chaijie 能力（独立 skill 或内置镜像） |
| 拆解这本书 / 拆解电影 / 拆短剧 / 分析作品 | 执行 chaijie 能力：优先 `chaijie-skill`，否则读取 `references/embedded-skills/chaijie-skill.md` |
| 提炼知识卡 / 出卡 / 萃取技法 / 更新知识库卡片 | 执行 tilian 能力：优先 `tilian-skill`，否则读取 `references/embedded-skills/tilian-skill.md` |
| 蒸馏大神 / 蒸馏作者 / 生成作者skill / 作者创作心智 | 执行 zhengliu 能力：优先 `zhengliu-skill`，否则读取 `references/embedded-skills/zhengliu-skill.md` |
| 安装大神skill / 安装作者skill | 运行 `install_skill.py` |
| 体检知识库 / 检查知识库 | 知识库体检 + skill 体检 |
| 进化知识库 / 进化skill / 升级skill | 体检后执行低风险修复，高风险列清单 |
| 清理知识库 / 删除临时文件 | 先列清单，再归档或按用户明确要求删除 |

## 子 skill 分工

### 子能力加载规则

路由到 `chaijie`、`tilian`、`zhengliu` 前，先用 `skills_list` 或 `skill_view` 判断独立子 skill 是否存在。

- 独立子 skill 存在：必须加载独立 skill，并按其流程执行。
- 独立子 skill 不存在：不得声称“没有该能力”；必须读取本 `zhishiku-skill` 内置镜像继续执行：
  - `references/embedded-skills/chaijie-skill.md`
  - `references/embedded-skills/tilian-skill.md`
  - `references/embedded-skills/zhengliu-skill.md`
- 内置镜像与独立子 skill 的目标一致：拆解进 `02`，提炼进 `03-07`，蒸馏进 `10`。
- 只有当内置镜像文件也缺失或损坏时，才停止并报告 skill 包不完整，不能自造低配流程替代。

### chaijie-skill

负责：把作品源文件变成 `02拆解报告` 中的分析笔记与综合报告。

### tilian-skill

负责：从 `02拆解报告` 产物中提炼 `03-07` 知识卡。

### zhengliu-skill

负责：基于 `02拆解报告` 中同一作者多部作品，蒸馏作者创作心智，并生成作者小 skill。

产物进入：

```text
08大神蒸馏/{作者名}/{skill-name}/
```

## 路由冲突处理

- 用户给的是作品源文件且要求“提炼”：先拆解，再提炼。
- 用户要求“蒸馏作者”，但 `02拆解报告` 中该作者作品少于 3 部：先提示证据不足，可做 A 级候选蒸馏，不建议生成 S 级作者 skill。
- 用户要求“清理/删除”：先列清单；只有用户明确说“物理删除这些文件”才删除。
