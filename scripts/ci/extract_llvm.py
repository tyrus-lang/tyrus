from __future__ import annotations

import argparse
import re
import tarfile
from pathlib import Path, PurePosixPath


LLVM_TOOLS = {
    "clang",
    "clang-23",
    "llvm-ar",
    "llvm-ar-23",
    "llvm-config",
    "llvm-config-23",
    "llvm-nm",
    "llvm-nm-23",
    "llvm-profdata",
    "llvm-profdata-23",
    "lld",
    "ld.lld",
    "ld64.lld",
    "lld-link",
    "llc",
    "llc-23",
    "wasm-ld",
    "wasm-ld-23",
}


def required_member(member: tarfile.TarInfo) -> bool:
    parts = PurePosixPath(member.name).parts
    if len(parts) < 2:
        return False
    relative = parts[1:]
    if relative[0] == "bin" and len(relative) == 2:
        name = relative[1]
        return (
            name in LLVM_TOOLS
            or name.removesuffix(".exe") in LLVM_TOOLS
            or name.lower().endswith(".dll")
        )
    if relative[0] == "include":
        return True
    if relative[0] == "lib":
        if len(relative) >= 3 and relative[1:3] == ("clang", "23"):
            return True
        if len(relative) == 2:
            name = relative[1]
            return bool(
                name.endswith((".a", ".lib", ".dylib"))
                or re.search(r"\.so(?:\.|$)", name)
            )
    return False


def safe_destination(root: Path, member: tarfile.TarInfo) -> Path:
    parts = PurePosixPath(member.name).parts
    if not parts or any(part in ("", ".", "..") for part in parts):
        raise ValueError(f"unsafe LLVM archive member: {member.name}")
    destination = root.joinpath(*parts)
    if not destination.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"LLVM archive member escapes extraction root: {member.name}")
    if member.issym():
        link = (destination.parent / member.linkname).resolve()
        if not link.is_relative_to(root.resolve()):
            raise ValueError(f"LLVM archive symlink escapes extraction root: {member.name}")
    if member.islnk():
        link_parts = PurePosixPath(member.linkname).parts
        if not link_parts or any(part in ("", ".", "..") for part in link_parts):
            raise ValueError(f"unsafe LLVM archive hard link: {member.name}")
        link = root.joinpath(*link_parts).resolve()
        if not link.is_relative_to(root.resolve()):
            raise ValueError(f"LLVM archive hard link escapes extraction root: {member.name}")
    return destination


def extract_required(archive: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, mode="r:xz") as source:
        for member in source:
            if not required_member(member):
                continue
            safe_destination(destination, member)
            source.extract(member, path=destination, filter="data")


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract only Tyrus-required LLVM files.")
    parser.add_argument("archive", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    extract_required(args.archive, args.destination)


if __name__ == "__main__":
    main()
