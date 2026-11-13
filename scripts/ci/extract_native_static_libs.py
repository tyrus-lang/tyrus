from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
from pathlib import Path


NATIVE_LIBS_LINE = re.compile(r"(?:^|\s)native-static-libs:\s*(.*)$")
ANSI_ESCAPE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")


def parse_native_static_libs(output: str) -> list[str]:
    output = ANSI_ESCAPE.sub("", output)
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
    if args.target == "x86_64-pc-windows-msvc":
        cargo_home = Path(os.environ.get("CARGO_HOME") or Path.home() / ".cargo")
        windows_libraries = sorted(
            {name for name in arguments if re.fullmatch(r"windows\.\d+\.\d+\.\d+\.lib", name)}
        )
        registry_sources = cargo_home / "registry" / "src"
        for library in windows_libraries:
            sources = list(
                registry_sources.glob(f"*/windows_x86_64_msvc-*/lib/{library}")
            )
            if len(sources) != 1:
                raise FileNotFoundError(
                    f"Expected one x86_64 MSVC import library `{library}` in "
                    f"{registry_sources}, found {len(sources)}"
                )
            shutil.copy2(sources[0], args.manifest.parent / library)
    manifest = {"target": args.target, "arguments": arguments}
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
