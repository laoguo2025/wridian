#!/usr/bin/env python3
"""Check knowledge health resources keep quality and operation gates visible."""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESOURCE_ROOT = ROOT.parent
HEALTH = ROOT / "scripts" / "health_check_knowledge_base.py"
INIT = ROOT / "scripts" / "init_knowledge_base.py"
SKILLS = RESOURCE_ROOT / "skills"
CALL_LOG_TEMPLATE = ROOT / "references" / "call-log-template.md"

REQUIRED_HEALTH_MARKERS = {
    "governance_files": "GOVERNANCE_FILES",
    "big_skill_markers": "BIG_SKILL_MARKERS",
    "recent_02_markers": "RECENT_02_MARKERS",
    "card_skill_rules": "CARD_SKILL_RULES",
    "frontmatter_validish": "frontmatter_validish",
    "missing_skill_parts": "missing_skill_parts",
    "check_big_skills": "check_big_skills",
    "check_recent_02_quality": "check_recent_02_quality",
    "check_recent_card_skill_shape": "check_recent_card_skill_shape",
    "check_call_log_quadrants": "check_call_log_quadrants",
    "check_card_evolution_age": "check_card_evolution_age",
    "extract_updated": "extract_updated",
    "version_record": "版本记录.md",
}

REQUIRED_INIT_MARKERS = {
    "governance_dir": "00知识库治理",
    "call_log": "调用记录台账.md",
    "call_log_template": "CALL_LOG",
}

REQUIRED_EMBEDDED_MARKERS = {
    "chaijie": {
        "path": SKILLS / "work-decompose" / "SKILL.md",
        "markers": {
            "quality_gate": "Phase 1.5：拆解质量闸门",
            "evidence_anchor": "证据锚点",
            "coverage": "覆盖率声明",
            "abc": "A 可提炼候选",
        },
        "max_lines": 540,
    },
    "tilian": {
        "path": SKILLS / "knowledge-card" / "SKILL.md",
        "markers": {
            "skill_gate": "知识卡 skill 化闸门",
            "input": "输入是什么",
            "logic": "处理逻辑是什么",
            "output": "输出结果是什么",
            "boundary": "失效边界是什么",
            "sab": "S/A/B 评级",
        },
        "max_lines": 430,
    },
    "zhengliu": {
        "path": SKILLS / "author-distill" / "SKILL.md",
        "markers": {
            "author_skill_gate": "作者 skill 化闸门",
            "self_contained": "自包含案例证据",
            "cross_work": "跨作品复现",
            "quality": "质量验证",
        },
        "max_lines": 430,
    },
}


def main() -> int:
    health_text = HEALTH.read_text(encoding="utf-8")
    init_text = INIT.read_text(encoding="utf-8")
    errors: list[str] = []

    for name, marker in REQUIRED_HEALTH_MARKERS.items():
        if marker not in health_text:
            errors.append(f"health script missing {name}: {marker}")

    for name, marker in REQUIRED_INIT_MARKERS.items():
        if marker not in init_text:
            errors.append(f"init script missing {name}: {marker}")

    if not CALL_LOG_TEMPLATE.is_file():
        errors.append(f"missing call log template: {CALL_LOG_TEMPLATE}")

    for embedded_name, spec in REQUIRED_EMBEDDED_MARKERS.items():
        path = spec["path"]
        if not path.exists():
            errors.append(f"embedded {embedded_name} missing: {path}")
            continue
        embedded_text = path.read_text(encoding="utf-8")
        for marker_name, marker in spec["markers"].items():
            if marker not in embedded_text:
                errors.append(f"embedded {embedded_name} missing {marker_name}: {marker}")
        embedded_lines = len(embedded_text.splitlines())
        max_lines = spec["max_lines"]
        if embedded_lines > max_lines:
            errors.append(f"embedded {embedded_name} too long: {embedded_lines} lines > {max_lines}")

    if errors:
        print("knowledge health resource check failed")
        for error in errors:
            print(f"- {error}")
        return 1

    print("knowledge health resource check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
