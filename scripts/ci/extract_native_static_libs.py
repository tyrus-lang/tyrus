from __future__ import annotations

import argparse
import json
import re
import shlex
from pathlib import Path


NATIVE_LIBS_LINE = re.compile(r"(?:^|\s)native-static-libs:\s*(.*)$")


def parse_native_static_libs(output: str) -> list[str]:
    matches = [
        match.group(1)
        for line in output.splitlines()
        if (match := NATIVE_LIBS_LINE.search(line)) is not None
    ]
    if not matches:
        raise ValueError("rustc output did not contain a native-static-libs line")
    arguments = shlex.split(matches[-1])
    if not arguments:
        raise ValueError("rustc reported an empty native-static-libs list")
    if len(arguments) > 256 or any(not argument or len(argument) > 4096 for argument in arguments):
        raise ValueError("rustc reported an invalid native-static-libs list")
    return arguments


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Extract rustc native link arguments into Tyrus's JSON manifest."
    )
    parser.add_argument("compiler_output", type=Path)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    arguments = parse_native_static_libs(args.compiler_output.read_text(encoding="utf-8-sig"))
    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest = {"target": args.target, "arguments": arguments}
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
