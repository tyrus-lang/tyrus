import io
import tarfile
import tempfile
import unittest
from pathlib import Path

from extract_llvm import extract_required, required_member


class LlvmArchiveExtractionTests(unittest.TestCase):
    def test_includes_windows_tool_dlls_and_excludes_unneeded_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            archive_path = root / "llvm.tar.xz"
            destination = root / "out"
            members = {
                "clang+llvm/bin/clang.exe": b"clang",
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
