import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from copy_windows_llvm_dependencies import copy_dependencies


class WindowsLlvmDependencyTests(unittest.TestCase):
    def test_vcpkg_libraries_supply_both_llvm_xml_names(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "vcpkg"
            source.mkdir()
            for name in ("zlib.lib", "zstd.lib", "libxml2.lib"):
                (source / name).write_bytes(name.encode())
            destination = root / "llvm"
            with contextlib.redirect_stdout(io.StringIO()):
                copy_dependencies(source, destination)
            self.assertEqual((destination / "zs.lib").read_bytes(), b"zlib.lib")
            self.assertEqual((destination / "zstd_static.lib").read_bytes(), b"zstd.lib")
            for name in ("xml2s.lib", "libxml2s.lib", "libxml2.lib"):
                self.assertEqual((destination / name).read_bytes(), b"libxml2.lib")

    def test_a_missing_dependency_is_rejected_before_rust_compilation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(FileNotFoundError, "zs.lib"):
                copy_dependencies(root, root / "llvm")
