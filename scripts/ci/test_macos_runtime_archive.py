from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


def llvm_tool(name: str) -> str | None:
    prefix = os.environ.get("LLVM_SYS_231_PREFIX")
    if prefix and (candidate := Path(prefix) / "bin" / name).is_file():
        return str(candidate)
    return shutil.which(name)


class MacosRuntimeArchiveTests(unittest.TestCase):
    @unittest.skipUnless(
        sys.platform == "darwin" and os.environ.get("LLVM_SYS_231_PREFIX"),
        "C++ ABI linking requires the extracted LLVM toolchain and macOS SDK",
    )
    def test_cpp_abi_symbols_are_resolved_before_compiling_llvm_consumers(self) -> None:
        clang, linker = llvm_tool("clang"), llvm_tool("ld64.lld")
        if not clang or not linker:
            self.fail("the LLVM installation must provide clang and ld64.lld")
        sdk_root = os.environ.get("SDKROOT")
        if not sdk_root or not Path(sdk_root, "usr", "lib").is_dir():
            self.fail("the macOS SDK root must be set before C++ ABI linking")
        with tempfile.TemporaryDirectory(prefix="tyrus-cxx-abi-") as directory:
            root = Path(directory)
            (root / "probe.cpp").write_text('''
namespace std { [[noreturn]] void terminate() noexcept; }
int initial_value() { return 7; }
int guarded_value() { static int value = initial_value(); return value; }
struct Abstract { virtual void call() = 0; virtual ~Abstract(); };
Abstract::~Abstract() {}
int main(int argc, char**) {
    if (argc == 100) std::terminate();
    return guarded_value() == 7 ? 0 : 1;
}
''', encoding="utf-8")
            output = subprocess.run([clang, f"--ld-path={linker}", "-isysroot", sdk_root, "probe.cpp", "-lc++", "-o", "linked"], cwd=root, capture_output=True, text=True)
            self.assertEqual(output.returncode, 0, output.stderr)
            subprocess.run([str(root / "linked")], check=True, capture_output=True)

    @unittest.skipUnless(
        sys.platform == "darwin" and os.environ.get("LLVM_SYS_231_PREFIX"),
        "Rust Mach-O linking requires the extracted LLVM toolchain and macOS SDK",
    )
    def test_rust_runtime_rlib_retains_jit_exports_with_one_allocator(self) -> None:
        clang = llvm_tool("clang")
        linker = llvm_tool("ld64.lld")
        nm = llvm_tool("llvm-nm")
        if not clang or not linker or not nm:
            self.fail("the LLVM installation must provide clang, ld64.lld, and llvm-nm")
        sdk_root = os.environ.get("SDKROOT")
        if not sdk_root or not Path(sdk_root, "usr", "lib").is_dir():
            self.fail("the macOS SDK root must be set before Rust Mach-O linking")
        with tempfile.TemporaryDirectory(prefix="tyrus-rust-jit-") as directory:
            root = Path(directory)
            (root / "runtime.rs").write_text('''
#[global_allocator]
static ALLOCATOR: std::alloc::System = std::alloc::System;
#[unsafe(no_mangle)]
pub extern "C" fn tyrus_probe() -> usize { vec![7_usize; 8].iter().sum() }
''', encoding="utf-8")
            (root / "main.rs").write_text("extern crate fixture_runtime as _; fn main() {}", encoding="utf-8")
            subprocess.run(["rustc", "--edition=2024", "--crate-name", "fixture_runtime", "--crate-type=rlib", "runtime.rs"], cwd=root, check=True, capture_output=True)
            output = subprocess.run([
                "rustc", "--edition=2024", "main.rs", "--extern", "fixture_runtime=libfixture_runtime.rlib",
                "-C", f"linker={clang}", "-C", f"link-arg=--ld-path={linker}",
                "-C", "link-arg=-isysroot", "-C", f"link-arg={sdk_root}",
                "-C", "link-arg=-Wl,-u,_tyrus_probe", "-o", "linked",
            ], cwd=root, capture_output=True, text=True)
            self.assertEqual(output.returncode, 0, output.stderr)
            symbols = subprocess.run([nm, "--defined-only", "linked"], cwd=root, check=True, capture_output=True, text=True).stdout
            self.assertIn("_tyrus_probe", symbols)
            subprocess.run([str(root / "linked")], check=True, capture_output=True)

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
