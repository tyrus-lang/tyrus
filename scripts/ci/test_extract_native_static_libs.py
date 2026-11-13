import unittest
import json
import tempfile
from pathlib import Path

from extract_native_static_libs import parse_native_static_libs


class NativeStaticLibParserTests(unittest.TestCase):
    def test_removes_rustc_colors_before_parsing_library_names(self) -> None:
        output = "\x1b[1m\x1b[36mnote\x1b[0m\x1b[1m: native-static-libs: -lm -lc\x1b[0m\r\n"
        self.assertEqual(parse_native_static_libs(output), ["-lm", "-lc"])

    def test_parses_linux_linker_flags(self) -> None:
        output = "note: native-static-libs: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc"
        self.assertEqual(
            parse_native_static_libs(output),
            ["-lgcc_s", "-lutil", "-lrt", "-lpthread", "-lm", "-ldl", "-lc"],
        )

    def test_preserves_macos_framework_argument_pairs(self) -> None:
        output = "native-static-libs: -framework Security -framework CoreFoundation -lc++"
        self.assertEqual(
            parse_native_static_libs(output),
            ["-framework", "Security", "-framework", "CoreFoundation", "-lc++"],
        )

    def test_parses_msvc_import_libraries(self) -> None:
        output = "note: native-static-libs: bcrypt.lib ws2_32.lib kernel32.lib"
        self.assertEqual(
            parse_native_static_libs(output),
            ["bcrypt.lib", "ws2_32.lib", "kernel32.lib"],
        )

    def test_rejects_missing_or_empty_native_library_lists(self) -> None:
        for output in ["rustc completed", "native-static-libs:"]:
            with self.subTest(output=output), self.assertRaises(ValueError):
                parse_native_static_libs(output)

    def test_manifest_records_the_target_triple(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            output = Path(temporary_directory) / "rustc-output.txt"
            manifest = Path(temporary_directory) / "tyrus-native-libs.json"
            output.write_text("native-static-libs: -lc\n", encoding="utf-8-sig")
            import subprocess
            import sys

            subprocess.run(
                [
                    sys.executable,
                    str(Path(__file__).with_name("extract_native_static_libs.py")),
                    str(output),
                    str(manifest),
                    "--target",
                    "aarch64-apple-darwin",
                ],
                check=True,
            )
            self.assertEqual(
                json.loads(manifest.read_text(encoding="utf-8")),
                {"target": "aarch64-apple-darwin", "arguments": ["-lc"]},
            )


if __name__ == "__main__":
    unittest.main()
