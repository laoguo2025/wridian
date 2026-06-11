#!/usr/bin/env python3
"""Install an author skill from 08大神蒸馏 into ~/.claude/skills."""

from __future__ import annotations

import argparse
import datetime as dt
import os
import shutil
from pathlib import Path


def copytree_replace(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(src, dst)


def main() -> int:
    parser = argparse.ArgumentParser(description="Install a distilled author skill")
    parser.add_argument("--source", required=True, help="source skill directory containing SKILL.md")
    parser.add_argument("--target-root", default="~/.claude/skills", help="Claude skills directory")
    parser.add_argument("--knowledge-root", default="", help="knowledge base root for logs/backups")
    parser.add_argument("--yes", action="store_true", help="overwrite existing target after backup")
    args = parser.parse_args()

    src = Path(os.path.expanduser(args.source)).resolve()
    if not src.is_dir():
        raise SystemExit(f"source is not a directory: {src}")
    if not (src / "SKILL.md").is_file():
        raise SystemExit(f"missing SKILL.md: {src}")

    target_root = Path(os.path.expanduser(args.target_root)).resolve()
    target_root.mkdir(parents=True, exist_ok=True)
    dst = target_root / src.name

    knowledge_root = Path(os.path.expanduser(args.knowledge_root)).resolve() if args.knowledge_root else None
    today = dt.date.today().isoformat()

    operation = "install"
    if dst.exists():
        if not args.yes:
            raise SystemExit(f"target exists, rerun with --yes after confirmation: {dst}")
        operation = "overwrite"
        if knowledge_root:
            backup = knowledge_root / "09文件归档" / f"skill备份-{today}" / dst.name
            backup.parent.mkdir(parents=True, exist_ok=True)
            copytree_replace(dst, backup)
        else:
            shutil.rmtree(dst)

    copytree_replace(src, dst)

    if knowledge_root:
        log = knowledge_root / "08大神蒸馏" / "_安装记录.md"
        log.parent.mkdir(parents=True, exist_ok=True)
        if not log.exists():
            log.write_text("# 安装记录\n\n| 时间 | Skill | 来源 | 目标 | 操作 |\n|---|---|---|---|---|\n", encoding="utf-8")
        with log.open("a", encoding="utf-8") as f:
            f.write(f"| {today} | {src.name} | {src} | {dst} | {operation} |\n")

    print(f"已安装: {dst}")
    print(f"操作: {operation}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
