#!/usr/bin/env python3
"""Normalize LLVM IR so that pure code movement compares as identical.

Unnamed temporaries are renumbered per function in order of first appearance
instead of being erased. Erasing them would hide a swapped pair of operands,
which is a real change; renumbering absorbs only the global counter drift that
moving code around causes. Unnamed basic blocks share that counter in LLVM, so
the `; <label>:N` markers are renumbered from the same map.

Symbolic names the code generator chooses (`if.then4`, `@fmt.1`) are kept on
purpose: they are emitted in builder order, so a change in them means the order
of emission changed, which is exactly what the snapshot exists to catch.

Usage: normalize-ir.py <file.ll> [...]   print the normalized text of each file
       normalize-ir.py --in-place <dir>  rewrite every .ll under a directory
"""

import re
import sys
from pathlib import Path

UNNAMED = re.compile(r"%(\d+)\b")
LABEL_MARKER = re.compile(r"^(\s*; <label>:)(\d+)(:?)")
DEFINE = re.compile(r"^define\b")
MODULE_ID = re.compile(r"^; ModuleID = .*$")
SOURCE_FILENAME = re.compile(r'^source_filename = ".*"$')
ABSOLUTE_PATH = re.compile(r'"/[^"]*"')


class Renumber:
    """Per function map from LLVM's unnamed slot number to a stable index."""

    def __init__(self) -> None:
        self.seen: dict[str, str] = {}

    def index(self, number: str) -> str:
        if number not in self.seen:
            self.seen[number] = f"t{len(self.seen)}"
        return self.seen[number]


def normalize(text: str) -> str:
    out = []
    mapping = Renumber()
    for line in text.split("\n"):
        if DEFINE.match(line):
            # Slot numbers restart at every function.
            mapping = Renumber()
        line = MODULE_ID.sub("; ModuleID = <normalized>", line)
        line = SOURCE_FILENAME.sub('source_filename = "<normalized>"', line)
        label = LABEL_MARKER.match(line)
        if label:
            rest = line[label.end() :]
            line = f"{label.group(1)}{mapping.index(label.group(2))}{label.group(3)}{rest}"
        line = UNNAMED.sub(lambda match: "%" + mapping.index(match.group(1)), line)
        line = ABSOLUTE_PATH.sub('"<path>"', line)
        out.append(line.rstrip())
    while out and not out[-1]:
        out.pop()
    return "\n".join(out) + "\n"


def main(argv: list[str]) -> int:
    if len(argv) >= 2 and argv[0] == "--in-place":
        root = Path(argv[1])
        count = 0
        for path in sorted(root.rglob("*.ll")):
            path.write_text(normalize(path.read_text()))
            count += 1
        print(f"normalized {count} files under {root}")
        return 0
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    for name in argv:
        sys.stdout.write(normalize(Path(name).read_text()))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
