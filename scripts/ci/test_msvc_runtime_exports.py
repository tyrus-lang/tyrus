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


def msvc_linker() -> str | None:
    tools_directory = os.environ.get("VCToolsInstallDir")
    if tools_directory:
        candidate = Path(tools_directory) / "bin" / "Hostx64" / "x64" / "link.exe"
        if candidate.is_file():
            return str(candidate)
    return llvm_tool("lld-link.exe") or llvm_tool("lld-link")


class MsvcRuntimeExportsTests(unittest.TestCase):
    def test_definition_file_pulls_exports_from_static_archive(self) -> None:
        if not os.environ.get("LLVM_SYS_231_PREFIX"):
            self.skipTest("the extracted release LLVM toolchain is not installed")
        clang = llvm_tool("clang") or llvm_tool("clang.exe")
        llvm_ar = llvm_tool("llvm-ar") or llvm_tool("llvm-ar.exe")
        linker = msvc_linker()
        if not clang or not llvm_ar or not linker:
            self.skipTest("COFF runtime export linking requires Clang, llvm-ar, and a linker")

        with tempfile.TemporaryDirectory(prefix="tyrus-msvc-exports-") as directory:
            root = Path(directory)
            (root / "runtime.c").write_text(
                "int tyrus_probe(void) { return 42; }\n", encoding="utf-8"
            )
            (root / "main.c").write_text(
                "void mainCRTStartup(void) {}\n", encoding="utf-8"
            )
            subprocess.run(
                [clang, "-target", "x86_64-pc-windows-msvc", "-c", "runtime.c", "-o", "runtime.obj"],
                cwd=root,
                check=True,
                capture_output=True,
                text=True,
            )
            subprocess.run(
                [clang, "-target", "x86_64-pc-windows-msvc", "-c", "main.c", "-o", "main.obj"],
                cwd=root,
                check=True,
                capture_output=True,
                text=True,
            )
            subprocess.run(
                [llvm_ar, "rcs", "runtime.lib", "runtime.obj"],
                cwd=root,
                check=True,
                capture_output=True,
                text=True,
            )
            (root / "runtime.def").write_text(
                "EXPORTS\ntyrus_probe\n", encoding="utf-8"
            )
            output = subprocess.run(
                [
                    linker,
                    "/subsystem:console",
                    "/entry:mainCRTStartup",
                    "/nodefaultlib",
                    f"/def:{root / 'runtime.def'}",
                    str(root / "main.obj"),
                    str(root / "runtime.lib"),
                    f"/out:{root / 'probe.exe'}",
                ],
                cwd=root,
                capture_output=True,
                text=True,
            )
            self.assertEqual(output.returncode, 0, output.stdout + output.stderr)
            self.assertTrue((root / "probe.exe").is_file())

    @unittest.skipUnless(os.name == "nt", "Rust COFF linking requires the Windows MSVC target")
    def test_definition_file_retains_an_export_from_a_rust_rlib(self) -> None:
        if not os.environ.get("LLVM_SYS_231_PREFIX"):
            self.skipTest("the extracted release LLVM toolchain is not installed")
        linker = msvc_linker()
        if not linker:
            self.fail("the MSVC or LLVM COFF linker was not found")

        with tempfile.TemporaryDirectory(prefix="tyrus-msvc-rust-exports-") as directory:
            root = Path(directory)
            (root / "runtime.rs").write_text(
                '#[global_allocator]\nstatic ALLOCATOR: std::alloc::System = std::alloc::System;\n#[unsafe(no_mangle)]\npub extern "C" fn tyrus_probe() -> i32 { 42 }\n',
                encoding="utf-8",
            )
            (root / "main.rs").write_text(
                "extern crate fixture_runtime as _; fn main() {}\n", encoding="utf-8"
            )
            (root / "runtime.def").write_text(
                "EXPORTS\ntyrus_probe\n", encoding="utf-8"
            )
            subprocess.run(
                ["rustc", "--edition=2024", "--crate-name", "fixture_runtime", "--crate-type=rlib", "runtime.rs"],
                cwd=root,
                check=True,
                capture_output=True,
                text=True,
            )
            output = subprocess.run(
                [
                    "rustc", "--edition=2024", "main.rs", "--extern", "fixture_runtime=libfixture_runtime.rlib",
                    "-C", f"linker={linker}", "-C", f"link-arg=/DEF:{root / 'runtime.def'}",
                    "-o", "linked.exe",
                ],
                cwd=root,
                capture_output=True,
                text=True,
            )
            self.assertEqual(output.returncode, 0, output.stdout + output.stderr)
            self.assertTrue((root / "linked.exe").is_file())


if __name__ == "__main__":
    unittest.main()
