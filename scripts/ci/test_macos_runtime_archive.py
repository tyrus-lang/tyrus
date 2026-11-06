from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


def llvm_tool(name: str) -> str | None:
    prefix = os.environ.get("LLVM_SYS_231_PREFIX")
    if prefix and (candidate := Path(prefix) / "bin" / name).is_file():
        return str(candidate)
    return shutil.which(name)


class MacosRuntimeArchiveTests(unittest.TestCase):
    def test_jit_export_is_retained_without_duplicating_a_native_dependency(self) -> None:
        tools = {name: llvm_tool(name) for name in ("clang", "llvm-ar", "llvm-nm", "ld64.lld")}
        if not all(tools.values()):
            self.skipTest("Mach-O archive linking requires Clang, LLVM ar/nm, and ld64.lld")
        with tempfile.TemporaryDirectory(prefix="tyrus-macho-link-") as directory:
            root = Path(directory)
            sources = {
                "main": "int dependency(void); int main(void) { return dependency(); }",
                "dependency": "int dependency(void) { return 0; }",
                "runtime": "int dependency(void); int tyrus_probe(void) { return dependency(); }",
            }
            for name, source in sources.items():
                (root / f"{name}.c").write_text(source, encoding="utf-8")
                subprocess.run(
                    [tools["clang"], "-target", "arm64-apple-macos14", "-c", f"{name}.c", "-o", f"{name}.o"],
                    cwd=root, check=True, capture_output=True,
                )
            for archive, members in (("dependency.a", ["dependency.o"]), ("runtime.a", ["runtime.o", "dependency.o"])):
                subprocess.run([tools["llvm-ar"], "rcs", archive, *members], cwd=root, check=True, capture_output=True)
            command = [tools["ld64.lld"], "-arch", "arm64", "-platform_version", "macos", "14.0", "14.0", "-e", "_main", "-u", "_tyrus_probe", "main.o", "dependency.a"]
            duplicate = subprocess.run([*command, "runtime.o", "dependency.o", "-o", "duplicate"], cwd=root, capture_output=True, text=True)
            self.assertNotEqual(duplicate.returncode, 0)
            self.assertIn("duplicate symbol", duplicate.stderr)
            subprocess.run([*command, "runtime.a", "-o", "linked"], cwd=root, check=True, capture_output=True)
            symbols = subprocess.run([tools["llvm-nm"], "--defined-only", "linked"], cwd=root, check=True, capture_output=True, text=True).stdout
            self.assertIn("_tyrus_probe", symbols)
            self.assertEqual(sum(line.split()[-1] == "_dependency" for line in symbols.splitlines() if line.split()), 1)


if __name__ == "__main__":
    unittest.main()
