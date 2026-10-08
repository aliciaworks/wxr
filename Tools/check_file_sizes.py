#!/usr/bin/env python3
"""No source file over 500 lines.

**Temporary rule.** It exists to stop a file growing past the point where a reader can hold it, which is a
fault that has to be split out afterwards and is much cheaper never to commit - and an agent writing code
does not feel the size of a file the way a person scrolling it does, so the size has to be checked
mechanically. The number is not sacred; it is small enough to be a habit. The rule goes when the remaining
files are under it and the habit is the reviewers', and until then a failure here is a file to split, not a
number to raise.

Usage: python3 Tools/check_file_sizes.py
"""

import sys
from pathlib import Path

LIMIT = 500

ROOT = Path(__file__).resolve().parent.parent
# Generated code is not written by hand and is not counted: a header-translator's output is as long as the
# header is.
SKIP = ("target", "generated", "webidl", "vendor", "sys")


def source_files():
    for path in (ROOT / "crates").rglob("*.rs"):
        if any(part in SKIP for part in path.parts):
            continue
        yield path
    for path in (ROOT / "Tools").rglob("*.rs"):
        yield path


def main() -> int:
    over = []
    for path in sorted(source_files()):
        lines = path.read_text(errors="ignore").count("\n")
        if lines > LIMIT:
            over.append((lines, path.relative_to(ROOT)))
    if over:
        print(f"{len(over)} file(s) over {LIMIT} lines - split them, do not raise the limit:")
        for lines, path in sorted(over, reverse=True):
            print(f"  {lines:5}  {path}")
        return 1
    print(f"no source file over {LIMIT} lines")
    return 0


if __name__ == "__main__":
    sys.exit(main())
