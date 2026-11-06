#!/usr/bin/env python3
"""Find the smallest stack the code generator needs to compile the corpus.

The generator recurses through the expression tree with a frame that carries
the locals of every arm of one very large `match`, so the stack it needs is a
consequence of how those functions are written. Splitting them should bring
this number down, and without a measurement taken first that gain is invisible.

This replaces measuring nesting depth: the parser refuses expressions deeper
than 30 levels, so the generator's stack is never reached that way, while real
programs still overflow the 2 MiB a libtest thread gets.

Usage: measure-stack.py <entries-file> [--ceiling MEGABYTES]
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

MEGABYTE = 1024 * 1024


def compiles_with(entries: str, stack_bytes: int) -> bool:
    with tempfile.TemporaryDirectory(prefix="tyrus-stack-probe-") as out:
        environment = dict(os.environ)
        environment["TYRUS_IR_SNAPSHOT_ENTRIES"] = entries
        environment["TYRUS_IR_SNAPSHOT_OUT"] = out
        environment["RUST_MIN_STACK"] = str(stack_bytes)
        result = subprocess.run(
            ["cargo", "test", "-p", "tyrus-codegen", "--test", "ir_snapshot",
             "--", "--ignored", "--quiet"],
            capture_output=True,
            text=True,
            env=environment,
        )
        return result.returncode == 0


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    entries = str(Path(argv[0]).resolve())
    ceiling = 256
    if "--ceiling" in argv:
        ceiling = int(argv[argv.index("--ceiling") + 1])

    high = ceiling * MEGABYTE
    if not compiles_with(entries, high):
        print(f"the corpus does not compile even with {ceiling} MiB of stack")
        return 1

    low = 0
    while high - low > MEGABYTE // 2:
        middle = (low + high) // 2
        if compiles_with(entries, middle):
            high = middle
        else:
            low = middle
    print(f"smallest stack that compiles the corpus: {high / MEGABYTE:.1f} MiB")
    print("(a libtest thread gets 2 MiB by default; the main thread gets 8 MiB)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
