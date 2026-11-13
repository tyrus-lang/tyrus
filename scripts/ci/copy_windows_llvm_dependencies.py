from __future__ import annotations

import argparse
import shutil
from pathlib import Path


LIBRARY_ALIASES = (
    ("zs.lib", "zlib.lib"),
    ("zstd_static.lib", "zstd.lib"),
    ("xml2s.lib", "libxml2s.lib", "libxml2.lib"),
)


def copy_dependencies(source: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    for aliases in LIBRARY_ALIASES:
        library = next((source / name for name in aliases if (source / name).is_file()), None)
        if library is None:
            raise FileNotFoundError(f"Missing static LLVM dependency in {source}: expected one of {', '.join(aliases)}")
        for name in aliases:
            target = destination / name
            shutil.copy2(library, target)
            print(f"LLVM dependency: {library} -> {target}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Copy vcpkg static libraries under LLVM release library names.")
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    copy_dependencies(args.source, args.destination)
