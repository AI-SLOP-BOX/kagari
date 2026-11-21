#!/usr/bin/env python3
"""Reject hand-written source files above the repository's 800-line limit."""

import os
from pathlib import Path

LIMIT = 800
EXTENSIONS = {".rs", ".ame", ".wgsl", ".py", ".sh"}
SKIP_DIRS = {".git", ".venv", "__pycache__", "node_modules", "target", "vendor"}


def main() -> int:
    oversized = []
    for root, directories, filenames in os.walk(Path.cwd()):
        directories[:] = [directory for directory in directories if directory not in SKIP_DIRS]
        for filename in filenames:
            path = Path(root) / filename
            if path.suffix not in EXTENSIONS:
                continue
            with path.open(encoding="utf-8") as source:
                line_count = sum(1 for _ in source)
            if line_count > LIMIT:
                oversized.append((line_count, path.relative_to(Path.cwd())))

    if oversized:
        for line_count, path in sorted(oversized, reverse=True):
            print(f"{line_count:5} {path}")
        return 1

    print(f"Source line limit ({LIMIT}) passes.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
