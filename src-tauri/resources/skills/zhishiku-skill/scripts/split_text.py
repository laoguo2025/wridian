#!/usr/bin/env python3
"""Split large md/txt files into readable chunks with a manifest."""

from __future__ import annotations

import argparse
import math
import os
from pathlib import Path


def split_by_paragraphs(text: str, max_chars: int) -> list[str]:
    paragraphs = text.splitlines(keepends=True)
    chunks: list[str] = []
    current: list[str] = []
    current_len = 0

    for paragraph in paragraphs:
        paragraph_len = len(paragraph)
        if current and current_len + paragraph_len > max_chars:
            chunks.append("".join(current).rstrip() + "\n")
            current = []
            current_len = 0
        current.append(paragraph)
        current_len += paragraph_len

    if current:
        chunks.append("".join(current).rstrip() + "\n")
    return chunks


def write_manifest(output_dir: Path, source: Path, chunks: list[str], suffix: str) -> None:
    lines = [
        "# 分卷清单",
        "",
        f"| 原文件 | {source} |",
        "|---|---|",
        f"| 分卷数 | {len(chunks)} |",
        f"| 总字数 | {sum(len(chunk) for chunk in chunks)} |",
        "",
        "## 分卷",
        "",
        "| 序号 | 文件 | 字数 |",
        "|---:|---|---:|",
    ]
    for index, chunk in enumerate(chunks, start=1):
        name = f"part-{index:03d}{suffix}"
        lines.append(f"| {index} | {name} | {len(chunk)} |")
    (output_dir / "manifest.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Split large md/txt files into chunks")
    parser.add_argument("--input", required=True, help="input md/txt file")
    parser.add_argument("--output-dir", required=True, help="directory for split files")
    parser.add_argument("--max-chars", type=int, default=30000, help="max characters per chunk")
    args = parser.parse_args()

    source = Path(os.path.expanduser(args.input)).resolve()
    if not source.is_file():
        raise SystemExit(f"input is not a file: {source}")
    if args.max_chars < 1000:
        raise SystemExit("--max-chars must be >= 1000")

    text = source.read_text(encoding="utf-8", errors="ignore")
    if not text.strip():
        raise SystemExit(f"input is empty after reading: {source}")

    output_dir = Path(os.path.expanduser(args.output_dir)).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    chunks = split_by_paragraphs(text, args.max_chars)
    suffix = source.suffix if source.suffix in {".md", ".txt"} else ".txt"
    width = max(3, math.ceil(math.log10(len(chunks) + 1)))

    for index, chunk in enumerate(chunks, start=1):
        path = output_dir / f"part-{index:0{width}d}{suffix}"
        path.write_text(chunk, encoding="utf-8")

    write_manifest(output_dir, source, chunks, suffix)
    print(f"输入文件: {source}")
    print(f"输出目录: {output_dir}")
    print(f"分卷数: {len(chunks)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
