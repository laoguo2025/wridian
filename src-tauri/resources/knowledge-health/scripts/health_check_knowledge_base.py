#!/usr/bin/env python3
"""Lightweight health check for knowledge base, big skills, and quality gates."""

from __future__ import annotations

import argparse
import csv
import os
import re
from datetime import date, datetime
from pathlib import Path
from typing import NamedTuple

REQUIRED_DIRS = [
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

CARD_DIRS = [
    "03故事模型",
    "04人物原型",
    "05情节方程",
    "06写作技法",
    "07综合素材",
]

GOVERNANCE_FILES = [
    "知识库使用说明.md",
    "00知识库治理/调用记录台账.md",
]

BIG_SKILL_MARKERS = {
    "chaijie-skill": [
        "Phase 1.5：拆解质量闸门",
        "证据锚点",
        "覆盖率声明",
        "候选分级",
    ],
    "tilian-skill": [
        "S/A/B",
        "编译来源",
        "关联索引",
        "内容忠实度抽查",
    ],
    "zhengliu-skill": [
        "自包含",
        "失效域",
        "验证分级",
        "本地路径",
    ],
}

RECENT_02_MARKERS = [
    "覆盖声明",
    "阅读方式",
    "结论权限",
    "证据锚点",
    "候选等级",
    "A/B/C",
]

CARD_SKILL_RULES = {
    "03故事模型": {
        "logic": ["核心运行逻辑", "运行逻辑", "基本结构", "模型"],
        "scene": ["适用题材", "适用场景", "调用阶段"],
        "output": ["输出", "大纲", "模型", "方案"],
        "boundary": ["失效", "边界", "区别", "生效条件"],
    },
    "04人物原型": {
        "logic": ["结构功能", "人物功能", "关系功能", "起点", "转折", "终点"],
        "scene": ["调用阶段", "人物设计", "适用场景"],
        "output": ["人物方案", "角色设计", "人物原型", "关系"],
        "boundary": ["失效", "边界", "不适用", "误用"],
    },
    "05情节方程": {
        "logic": ["基本结构", "情节功能", "前提", "动作", "结果", "公式"],
        "scene": ["调用阶段", "分场写作", "适用场景"],
        "output": ["情节", "桥段", "方程", "场景"],
        "boundary": ["失效", "边界", "生效条件", "注意事项"],
    },
    "06写作技法": {
        "logic": ["操作步骤", "使用方法", "创作要点", "核心问题", "组合", "调用顺序", "体系", "审美判断", "一句话法则"],
        "scene": ["适用场景", "调用阶段", "使用场景", "适用题材", "审美判断"],
        "output": ["诊断", "技法", "桥段", "句段", "场景", "方案", "体系", "路线", "判断", "法则", "调用摘要"],
        "boundary": ["注意事项", "失效", "边界", "不适用", "冲突", "误判", "失效边界"],
    },
    "07综合素材": {
        "logic": ["调用方向", "戏剧内核", "使用方法", "素材本体"],
        "scene": ["调用阶段", "细节填充", "调用方向"],
        "output": ["素材", "情境", "设定", "细节"],
        "boundary": ["失效", "边界", "不适用", "误用"],
    },
}

GOOD_PERFORMANCE = {"命中", "可用"}
BAD_PERFORMANCE = {"勉强", "误导"}
HIGH_FREQ_THRESHOLD = 3
STALE_DAYS = 180
REVIEW_MARKERS = (
    "体检状态",
    "治理状态",
    "核查状态",
    "review_status",
    "governance_status",
    "冲突对象",
    "冲突卡片",
    "conflicts_with",
    "不确定性",
    "待核查",
    "uncertainty",
    "[!contradiction]",
    "[!gap]",
)


class CallRecord(NamedTuple):
    date: str
    work: str
    card: str
    frequency: int
    recent: str
    performance: str
    evolution: str


def check_root(root: Path) -> list[str]:
    issues: list[str] = []
    for d in REQUIRED_DIRS:
        if not (root / d).is_dir():
            issues.append(f"缺一级目录: {d}")
    for f in ["08大神蒸馏/大神索引.md", "08大神蒸馏/_安装记录.md"]:
        if not (root / f).is_file():
            issues.append(f"缺基础文件: {f}")
    for f in GOVERNANCE_FILES:
        if not (root / f).is_file():
            issues.append(f"缺治理入口: {f}")
    return issues


def has_frontmatter_description(text: str) -> bool:
    if not text.startswith("---") or text.count("---") < 2:
        return False
    frontmatter = text.split("---", 2)[1]
    return "description:" in frontmatter


def check_distilled_skills(root: Path) -> list[str]:
    issues: list[str] = []
    base = root / "08大神蒸馏"
    if not base.is_dir():
        return issues
    for skill_md in base.glob("*/*/SKILL.md"):
        text = skill_md.read_text(encoding="utf-8", errors="ignore")
        if not has_frontmatter_description(text):
            issues.append(f"作者skill缺description: {skill_md}")
        if str(root) in text:
            issues.append(f"作者skill含本地绝对路径依赖: {skill_md}")
        if not (skill_md.parent / "版本记录.md").is_file():
            issues.append(f"作者skill缺版本记录.md: {skill_md.parent.relative_to(root)}")
    return issues


def check_big_skills(root: Path) -> list[str]:
    issues: list[str] = []
    base = root / "BB 技能库"
    for skill_name, markers in BIG_SKILL_MARKERS.items():
        skill_md = base / skill_name / "SKILL.md"
        if not skill_md.is_file():
            issues.append(f"缺大skill: {skill_md}")
            continue
        text = skill_md.read_text(encoding="utf-8", errors="ignore")
        if not has_frontmatter_description(text):
            issues.append(f"大skill缺description: {skill_md}")
        for marker in markers:
            if marker not in text:
                issues.append(f"大skill缺质量标记: {skill_name} -> {marker}")
    return issues


def check_recent_02_quality(root: Path, limit: int = 12) -> list[str]:
    issues: list[str] = []
    base = root / "02拆解报告"
    if not base.is_dir():
        return issues

    files = [
        p for p in base.rglob("*.md")
        if any(key in p.name for key in ("分析笔记", "综合报告", "案例分析报告"))
    ]
    files = sorted(files, key=lambda p: p.stat().st_mtime, reverse=True)[:limit]
    if not files:
        issues.append("02缺可抽检拆解产物")
        return issues

    weak = []
    for p in files:
        text = p.read_text(encoding="utf-8", errors="ignore")
        hits = sum(1 for marker in RECENT_02_MARKERS if marker in text)
        if hits < 2:
            weak.append(str(p.relative_to(root)))

    if weak:
        issues.append(f"最近02拆解产物缺质量闸门信号: {len(weak)}/{len(files)}")
        for rel in weak[:5]:
            issues.append(f"  - {rel}")
    return issues


def frontmatter_validish(text: str) -> bool:
    if not text.startswith("---"):
        return False
    lines = text.splitlines()
    close_at = None
    for i, line in enumerate(lines[:80]):
        if i > 0 and line.strip() == "---":
            close_at = i
            break
    if close_at is None:
        return False
    body = lines[1:close_at]
    if any("---" in line for line in body):
        return False
    if any(line.startswith("|") for line in body):
        return False
    return True


def card_board(path: Path, root: Path) -> str | None:
    try:
        rel = path.relative_to(root)
    except ValueError:
        return None
    return rel.parts[0] if rel.parts else None


def missing_skill_parts(text: str, board: str) -> list[str]:
    rules = CARD_SKILL_RULES.get(board)
    if not rules:
        return []
    missing = []
    for part, markers in rules.items():
        if not any(marker in text for marker in markers):
            missing.append(part)
    return missing


def check_recent_card_skill_shape(root: Path, limit: int = 20) -> list[str]:
    issues: list[str] = []
    files = []
    for dirname in CARD_DIRS:
        base = root / dirname
        if base.is_dir():
            files.extend(base.rglob("*.md"))

    files = sorted(files, key=lambda p: p.stat().st_mtime, reverse=True)[:limit]
    broken_frontmatter = []
    weak = []
    for p in files:
        text = p.read_text(encoding="utf-8", errors="ignore")
        rel = str(p.relative_to(root))
        if not frontmatter_validish(text):
            broken_frontmatter.append(rel)
        board = card_board(p, root)
        if board:
            missing = missing_skill_parts(text, board)
            if missing:
                weak.append(f"{rel} 缺 {','.join(missing)}")

    if broken_frontmatter:
        issues.append(f"最近03-07知识卡frontmatter疑似损坏: {len(broken_frontmatter)}/{len(files)}")
        for rel in broken_frontmatter[:5]:
            issues.append(f"  - {rel}")
    if weak:
        issues.append(f"最近03-07知识卡skill化能力不足: {len(weak)}/{len(files)}")
        for rel in weak[:5]:
            issues.append(f"  - {rel}")
    return issues


def extract_updated(text: str) -> date | None:
    match = re.search(r"^updated:\s*['\"]?(\d{4}-\d{2}-\d{2})", text, re.MULTILINE)
    if not match:
        return None
    try:
        return datetime.strptime(match.group(1), "%Y-%m-%d").date()
    except ValueError:
        return None


def check_card_evolution_age(root: Path, limit: int = 80) -> list[str]:
    issues: list[str] = []
    files = []
    for dirname in CARD_DIRS:
        base = root / dirname
        if base.is_dir():
            files.extend(base.rglob("*.md"))

    files = sorted(files, key=lambda p: p.stat().st_mtime, reverse=True)[:limit]
    missing_updated = []
    stale = []
    today = date.today()
    for p in files:
        text = p.read_text(encoding="utf-8", errors="ignore")
        rel = str(p.relative_to(root))
        updated = extract_updated(text)
        if updated is None:
            missing_updated.append(rel)
            continue
        age = (today - updated).days
        if age > STALE_DAYS:
            stale.append(f"{rel} {age}天未进化")

    if missing_updated:
        issues.append(f"最近03-07知识卡缺updated: {len(missing_updated)}/{len(files)}")
        for rel in missing_updated[:5]:
            issues.append(f"  - {rel}")
    if stale:
        issues.append(f"最近03-07知识卡超过{STALE_DAYS}天未进化: {len(stale)}/{len(files)}")
        for rel in stale[:5]:
            issues.append(f"  - {rel}")
    return issues


def parse_call_log(root: Path) -> list[CallRecord]:
    path = root / "00知识库治理" / "调用记录台账.md"
    if not path.is_file():
        return []

    records: list[CallRecord] = []
    for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = line.strip()
        if not line.startswith("|") or "---" in line or "日期" in line:
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 9 or not cells[0] or not cells[2]:
            continue
        try:
            freq = int(cells[3]) if cells[3] else 0
        except ValueError:
            freq = 0
        records.append(
            CallRecord(
                date=cells[0],
                work=cells[1],
                card=cells[2],
                frequency=freq,
                recent=cells[4],
                performance=cells[5],
                evolution=cells[7],
            )
        )
    return records


def classify_record(record: CallRecord) -> tuple[str, str]:
    high_freq = record.frequency >= HIGH_FREQ_THRESHOLD
    good = record.performance in GOOD_PERFORMANCE
    bad = record.performance in BAD_PERFORMANCE

    if high_freq and good:
        return "杠杆", "升金卡/重点维护/产品化"
    if high_freq and bad:
        return "内耗", "优先重写/降评级/合并"
    if not high_freq and good:
        return "浪费", "补入口/补关联卡/补适用场景"
    if not high_freq and bad:
        return "垃圾", "废弃/封存/合并"
    return "待验证", "安排真实调用/补调用表现"


def check_call_log_quadrants(root: Path) -> list[str]:
    issues: list[str] = []
    records = parse_call_log(root)
    if not records:
        issues.append("调用记录台账无有效记录，无法计算质量×频率四象限")
        return issues

    buckets: dict[str, list[tuple[CallRecord, str]]] = {}
    for record in records:
        quadrant, action = classify_record(record)
        buckets.setdefault(quadrant, []).append((record, action))
        if not record.evolution:
            issues.append(f"调用记录缺进化方向: {record.card}")

    print("质量×频率四象限:")
    for quadrant in ("杠杆", "浪费", "内耗", "垃圾", "待验证"):
        items = buckets.get(quadrant, [])
        print(f"- {quadrant}: {len(items)}")
        for record, action in items[:3]:
            print(f"  - {record.card} | {record.performance} | {record.frequency} | {action}")
    return issues


def collect_review_marks(root: Path, limit: int = 80) -> list[str]:
    marks: list[str] = []
    for dirname in CARD_DIRS:
        base = root / dirname
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.md")):
            text = path.read_text(encoding="utf-8", errors="ignore")
            hit = next((marker for marker in REVIEW_MARKERS if marker in text), None)
            if hit:
                marks.append(f"{path.relative_to(root)} | {hit}")
            if len(marks) >= limit:
                return marks
    return marks


def write_health_report(root: Path, issues: list[str], review_marks: list[str]) -> Path:
    report_dir = root / "00知识库治理"
    report_dir.mkdir(parents=True, exist_ok=True)
    checked_at = datetime.now().astimezone().isoformat(timespec="seconds")
    stamp = "".join(ch for ch in checked_at if ch.isalnum())
    report_path = report_dir / f"知识库体检-{stamp}.md"
    highest = issues[0] if issues else "无阻断问题"
    lines = [
        f"# 知识库体检 {checked_at}",
        "",
        "## 总览",
        f"- 问题总数：{len(issues)}",
        f"- 最高风险：{highest}",
        "- 自动修复项：本脚本只生成报告，不自动改写语义字段。",
        "- 待确认动作：冲突、合并、删除、语义重写均需用户确认。",
        "",
        "## 问题清单",
    ]
    lines.extend([f"- {issue}" for issue in issues] or ["- 暂无基础体检问题。"])
    lines.extend(["", "## 冲突与不确定性标记"])
    lines.extend([f"- {mark}" for mark in review_marks] or ["- 暂无已标记冲突或不确定性。"])
    lines.extend([
        "",
        "## 字段协议",
        "- 中文字段：`体检状态`、`冲突对象`、`不确定性`。",
        "- 兼容字段：`review_status`、`conflicts_with`、`uncertainty`。",
    ])
    report_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return report_path


def main() -> int:
    parser = argparse.ArgumentParser(description="Health check knowledge base")
    parser.add_argument("--root", default=os.environ.get("WRIDIAN_KNOWLEDGE_ROOT"))
    parser.add_argument("--report", action="store_true", help="write 00知识库治理/知识库体检-YYYYMMDDTHHMMSS*.md")
    args = parser.parse_args()
    if not args.root:
        parser.error("pass --root or set WRIDIAN_KNOWLEDGE_ROOT")

    root = Path(os.path.expanduser(args.root)).resolve()
    issues = (
        check_root(root)
        + check_big_skills(root)
        + check_distilled_skills(root)
        + check_recent_02_quality(root)
        + check_recent_card_skill_shape(root)
        + check_card_evolution_age(root)
        + check_call_log_quadrants(root)
    )
    review_marks = collect_review_marks(root)
    if args.report:
        report_path = write_health_report(root, issues, review_marks)
        print(f"体检报告: {report_path}")

    if issues:
        print(f"发现问题: {len(issues)}")
        for issue in issues:
            print(f"- {issue}")
        return 1

    if review_marks:
        print(f"冲突/待核查标记: {len(review_marks)}")
        for mark in review_marks[:8]:
            print(f"- {mark}")
    print("基础体检通过")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
