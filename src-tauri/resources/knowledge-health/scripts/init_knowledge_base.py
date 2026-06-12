#!/usr/bin/env python3
"""Initialize a Wridian-style knowledge base."""

from __future__ import annotations

import argparse
import os
from pathlib import Path

FOLDERS = [
    "00知识库治理",
    "01原始资料",
    "02拆解报告",
    "03故事模型",
    "04人物原型",
    "05情节方程",
    "06写作技法",
    "07综合素材",
    "08大神蒸馏",
    "09文件归档",
]

HEALTH_CHECK = r'''#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"

required=(
  "00知识库治理"
  "01原始资料"
  "02拆解报告"
  "03故事模型"
  "04人物原型"
  "05情节方程"
  "06写作技法"
  "07综合素材"
  "08大神蒸馏"
  "09文件归档"
)

issues=0
echo "========================================"
echo "  知识库 · 基础一致性校验"
echo "========================================"
echo

echo "── 1. 一级目录 ──"
for d in "${required[@]}"; do
  if [[ -d "$ROOT/$d" ]]; then
    echo "✓ $d"
  else
    echo "✗ 缺目录 → $d"
    issues=$((issues + 1))
  fi
done

echo
echo "── 2. 大神蒸馏基础文件 ──"
for f in "08大神蒸馏/大神索引.md" "08大神蒸馏/_安装记录.md"; do
  if [[ -f "$ROOT/$f" ]]; then
    echo "✓ $f"
  else
    echo "✗ 缺文件 → $f"
    issues=$((issues + 1))
  fi
done

echo
if [[ "$issues" -eq 0 ]]; then
  echo "✓ 基础校验通过"
  exit 0
else
  echo "✗ 发现 $issues 个基础问题"
  exit 1
fi
'''

USAGE = """# 知识库使用说明

## 常用口令

```text
搭建知识库
拆解作品
提炼知识卡
蒸馏大神作者
安装大神skill
体检知识库
进化skill
清理知识库
```

## 一级目录

- `00知识库治理`：调用记录台账。
- `01原始资料`：待处理素材。
- `02拆解报告`：作品拆解笔记与综合报告。
- `03故事模型`：可复用故事运行机制。
- `04人物原型`：人物位置、关系功能和精神内核。
- `05情节方程`：场景、桥段和情绪触发公式。
- `06写作技法`：可执行写作技法、组合流程和审美法则。
- `07综合素材`：设定、道具、机构、术语、场景和金句。
- `08大神蒸馏`：作者方法论和可复用 skill。
- `09文件归档`：备份、迁移记录、待清理文件和旧版本。

## 知识治理

1. 文件系统是唯一事实来源。
2. 用户可以增、改、删分类目录，体检时按实际目录修正。
3. 知识卡可以被多个作品引用，但不会自动变成作品记忆。
4. 从知识到作品，通过引用、采纳或改写成作品设定进入项目。
5. 从作品到知识，通过摘录、抽象或沉淀为知识卡离开项目。

## 知识卡结构

知识卡应写清四件事：

- 输入：什么场景、材料或问题可以调用这张卡。
- 处理逻辑：卡片如何判断、拆解或生成方案。
- 输出：调用后能得到什么结果。
- 边界：什么情况下会失效、误用或需要回源复查。

## 旧目录处理

发现旧版 00-11、重名目录或已被合并的分类时，先迁移到当前 00-09 结构；不能确认归属的文件先放入 `09文件归档`，不要直接物理删除。

## 重要规则

- 拆解产物进 `02拆解报告`。
- 知识卡进 `03-07`，只保留 S 级。
- 作者 skill 由蒸馏流程生成，存入 `08大神蒸馏`。
- 清理默认归档，不直接物理删除。
"""

DASHEN_INDEX = """# 大神索引

记录由 `zhengliu-skill` 蒸馏出的作者小 skill。

| 作者 | Skill | 状态 | 安装位置 | 更新时间 |
|---|---|---|---|---|
"""

INSTALL_LOG = """# 安装记录

记录从 `08大神蒸馏` 安装到 `~/.claude/skills/` 的作者小 skill。

| 时间 | Skill | 来源 | 目标 | 操作 |
|---|---|---|---|---|
"""

CALL_LOG = """# Wridian知识库 · 调用记录台账

> 本台账记录知识卡被真实使用后的表现，用于统计调用频率、最近调用、调用表现和进化方向。字段是管理台账字段，不是知识卡 frontmatter。

## 记录规则

1. 只有知识卡被真实用于拆解、创作、诊断、改写或方案判断时，才记录一次调用。
2. 同一任务里同一张知识卡多次被参考，默认记为一次；若不同环节发挥不同作用，可以拆成多条。
3. 调用表现只看本次任务效果，不看卡片文字是否漂亮。
4. 进化方向必须写成可执行动作，避免只写“优化”“完善”。
5. 如果一张卡误导判断，必须记录，后续全库体检优先处理。

## 调用记录表

| 日期 | 调用作品 | 知识卡 | 调用频率 | 最近调用 | 调用表现 | 关联卡片 | 进化方向 | 备注 |
|---|---|---|---:|---|---|---|---|---|
|  |  |  |  |  |  |  |  |  |

## 字段说明

| 字段 | 填写方式 |
|---|---|
| 日期 | 本次调用发生日期，格式建议 `YYYY-MM-DD`。 |
| 调用作品 | 本次用于哪个作品、项目、桥段、人物方案或拆解任务。 |
| 知识卡 | 被调用的知识卡路径或标题。 |
| 调用频率 | 该知识卡截至本次的累计调用次数。 |
| 最近调用 | 该知识卡最近一次真实调用日期，通常与本条日期一致。 |
| 调用表现 | 填 `命中`、`可用`、`勉强`、`误导`。 |
| 关联卡片 | 本次一起命中、互相补充或发生冲突的卡片。 |
| 进化方向 | 填 `补案例`、`拆短卡`、`合并`、`升评级`、`降评级`、`待复审`、`废弃` 或更具体动作。 |
| 备注 | 简短说明为什么表现好或不好。 |

## 质量 × 频率四象限

| 质量 | 频率 | 结果 | 处理原则 |
|---|---|---|---|
| 低质量 | 低频 | 垃圾 | 废弃、合并或封存。 |
| 低质量 | 高频 | 内耗 | 优先重写或降评级。 |
| 高质量 | 低频 | 浪费 | 补入口、补关联卡片、补适用场景。 |
| 高质量 | 高频 | 杠杆 | 评为金卡，重点维护，优先产品化和系统化。 |
"""


def write_if_missing(path: Path, content: str, executable: bool = False) -> bool:
    if path.exists():
        return False
    path.write_text(content, encoding="utf-8")
    if executable:
        path.chmod(path.stat().st_mode | 0o755)
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description="Initialize knowledge base")
    parser.add_argument("--root", default="~/Desktop/Wridian知识库", help="target knowledge base root")
    args = parser.parse_args()

    root = Path(os.path.expanduser(args.root)).resolve()
    root.mkdir(parents=True, exist_ok=True)

    created_dirs = 0
    created_files = 0

    for folder in FOLDERS:
        path = root / folder
        if not path.exists():
            path.mkdir(parents=True)
            created_dirs += 1

    created_files += write_if_missing(root / "health-check.sh", HEALTH_CHECK, executable=True)
    created_files += write_if_missing(root / "知识库使用说明.md", USAGE)
    created_files += write_if_missing(root / "00知识库治理" / "调用记录台账.md", CALL_LOG)
    created_files += write_if_missing(root / "08大神蒸馏" / "大神索引.md", DASHEN_INDEX)
    created_files += write_if_missing(root / "08大神蒸馏" / "_安装记录.md", INSTALL_LOG)

    print(f"知识库路径: {root}")
    print(f"新建目录: {created_dirs}")
    print(f"新建文件: {created_files}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
