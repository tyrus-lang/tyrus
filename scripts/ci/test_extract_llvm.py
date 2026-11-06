import io
import os
import tarfile
import tempfile
import unittest
from pathlib import Path

from extract_llvm import extract_required, required_member


class LlvmArchiveExtractionTests(unittest.TestCase):
    def test_keeps_profile_tools_required_by_release_packages(self) -> None:
        for name in ["llvm-profdata", "llvm-profdata-23", "llvm-profdata.exe"]:
            self.assertTrue(required_member(tarfile.TarInfo(f"llvm/bin/{name}")))

    def test_includes_platform_lld_drivers(self) -> None:
        for name in ["ld64.lld", "ld.lld", "lld-link.exe"]:
            with self.subTest(name=name):
                self.assertTrue(required_member(tarfile.TarInfo(f"llvm/bin/{name}")))

    @unittest.skipIf(os.name == "nt", "Mach-O archive symlinks are extracted on Unix hosts")
    def test_preserves_macos_linker_symlink_and_its_executable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            archive_path = root / "llvm.tar.xz"
            with tarfile.open(archive_path, mode="w:xz") as archive:
                executable = tarfile.TarInfo("llvm/bin/lld")
                executable.mode = 0o755
                executable.size = len(b"linker")
                archive.addfile(executable, io.BytesIO(b"linker"))
                link = tarfile.TarInfo("llvm/bin/ld64.lld")
                link.type = tarfile.SYMTYPE
                link.linkname = "lld"
                archive.addfile(link)
            destination = root / "out"
            extract_required(archive_path, destination)
            linker = destination / "llvm/bin/ld64.lld"
            self.assertTrue(linker.is_symlink())
            self.assertEqual(linker.read_bytes(), b"linker")
            self.assertTrue(linker.stat().st_mode & 0o111)

    def test_includes_windows_tool_dlls_and_excludes_unneeded_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            archive_path = root / "llvm.tar.xz"
            destination = root / "out"
            members = {
                "clang+llvm/bin/clang.exe": b"clang",
                "clang+llvm/bin/llvm-profdata.exe": b"profile merger",
                "clang+llvm/bin/LLVM-C.dll": b"llvm",
                "clang+llvm/bin/readme.txt": b"not required",
                "clang+llvm/lib/LLVM-C.lib": b"import library",
            }
            with tarfile.open(archive_path, mode="w:xz") as archive:
                for name, content in members.items():
                    info = tarfile.TarInfo(name)
                    info.size = len(content)
                    archive.addfile(info, io.BytesIO(content))

            extract_required(archive_path, destination)

            self.assertEqual(
                (destination / "clang+llvm/bin/clang.exe").read_bytes(), b"clang"
            )
            self.assertEqual(
                (destination / "clang+llvm/bin/llvm-profdata.exe").read_bytes(), b"profile merger"
            )
            self.assertEqual(
                (destination / "clang+llvm/bin/LLVM-C.dll").read_bytes(), b"llvm"
            )
            self.assertFalse((destination / "clang+llvm/bin/readme.txt").exists())
            self.assertEqual(
                (destination / "clang+llvm/lib/LLVM-C.lib").read_bytes(),
                b"import library",
            )

    def test_requires_only_dlls_directly_under_bin(self) -> None:
        self.assertTrue(required_member(tarfile.TarInfo("llvm/bin/LLVM-C.dll")))
        self.assertFalse(required_member(tarfile.TarInfo("llvm/bin/subdir/LLVM-C.dll")))
        self.assertFalse(required_member(tarfile.TarInfo("llvm/lib/LLVM-C.dll")))


if __name__ == "__main__":
    unittest.main()
