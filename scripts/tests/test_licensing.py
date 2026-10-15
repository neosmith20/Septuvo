"""Tests for scripts/check_licensing.py (D-033). Offline; uses temporary copies."""

import shutil
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
sys.path.insert(0, str(SCRIPTS))

import check_licensing as C  # noqa: E402

FILES = ["LICENSE", "LICENSING.md", "NOTICE", "licensing.toml", "Cargo.toml",
         "LICENSES/PolyForm-Noncommercial-1.0.0.md", "brand/NOTICE.md",
         "crates/septuvo-policy/Cargo.toml", "crates/septuvo-cli/Cargo.toml"]

POLYFORM_CRATE = """[package]
name = "septuvo-vpn-core"
version.workspace = true
edition.workspace = true
license = "PolyForm-Noncommercial-1.0.0"
publish = false
"""


class Fixture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        for f in FILES:
            (self.root / f).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(REPO_ROOT / f, self.root / f)

    def tearDown(self):
        self.tmp.cleanup()

    def edit(self, rel, old, new):
        p = self.root / rel
        text = p.read_text(encoding="utf-8")
        self.assertIn(old, text, f"fixture text not found in {rel}")
        p.write_text(text.replace(old, new), encoding="utf-8")

    def errors(self):
        return C.check(self.root)

    def assert_error(self, fragment):
        errs = self.errors()
        self.assertTrue(any(fragment in e for e in errs), f"{fragment!r} not in {errs}")

    def add_polyform_crate(self, manifest=POLYFORM_CRATE, notice=True, license_copy=True):
        crate = self.root / "crates/septuvo-vpn-core"
        crate.mkdir(parents=True)
        (crate / "Cargo.toml").write_text(manifest, encoding="utf-8")
        if license_copy:
            shutil.copy(self.root / C.POLYFORM_TEXT, crate / "LICENSE.md")
        if notice:
            (crate / "NOTICE").write_text("Required Notice: Copyright 2026 the Septuvo rights holder\n", encoding="utf-8")
        self.edit("Cargo.toml", '"crates/septuvo-cli"]', '"crates/septuvo-cli", "crates/septuvo-vpn-core"]')
        with open(self.root / "licensing.toml", "a", encoding="utf-8") as f:
            f.write('\n[[crate]]\npath = "crates/septuvo-vpn-core"\nlicense = "PolyForm-Noncommercial-1.0.0"\ncomponent = "vpn"\n')
        with open(self.root / "LICENSING.md", "a", encoding="utf-8") as f:
            f.write("\ncrates/septuvo-vpn-core\n")


class Checks(Fixture):
    def test_repository_passes(self):
        self.assertEqual(C.check(REPO_ROOT), [])
        self.assertEqual(self.errors(), [])

    def test_altered_apache_license_fails(self):
        self.edit("LICENSE", "Version 2.0", "Version 2.1")
        self.assert_error("unmodified Apache License 2.0")

    def test_altered_polyform_text_fails(self):
        self.edit(C.POLYFORM_TEXT, "Any noncommercial purpose", "Any purpose")
        self.assert_error("official PolyForm")

    def test_workspace_default_license_fails(self):
        self.edit("Cargo.toml", "[workspace.package]\n", '[workspace.package]\nlicense = "Apache-2.0"\n')
        self.assert_error("must not set `license`")

    def test_unclassified_new_crate_fails(self):
        (self.root / "crates/septuvo-server").mkdir(parents=True)
        (self.root / "crates/septuvo-server/Cargo.toml").write_text(
            '[package]\nname = "septuvo-server"\nlicense = "Apache-2.0"\n', encoding="utf-8")
        self.edit("Cargo.toml", '"crates/septuvo-cli"]', '"crates/septuvo-cli", "crates/septuvo-server"]')
        self.assert_error("not classified in licensing.toml")

    def test_inherited_or_mismatched_license_fails(self):
        self.edit("crates/septuvo-cli/Cargo.toml", 'license = "Apache-2.0"', "license.workspace = true")
        self.assert_error("must be set explicitly")
        self.edit("crates/septuvo-cli/Cargo.toml", "license.workspace = true", 'license = "MIT"')
        self.assert_error("differs from licensing.toml")

    def test_unknown_license_in_map_fails(self):
        self.edit("licensing.toml", 'path = "crates/septuvo-cli"\nlicense = "Apache-2.0"',
                  'path = "crates/septuvo-cli"\nlicense = "MIT"')
        self.assert_error("is not Apache-2.0 or PolyForm")

    def test_proper_polyform_crate_passes(self):
        self.add_polyform_crate()
        self.assertEqual(self.errors(), [])

    def test_polyform_crate_must_not_be_publishable(self):
        self.add_polyform_crate(POLYFORM_CRATE.replace("publish = false\n", ""))
        self.assert_error("publish = false")

    def test_polyform_crate_needs_notice_and_license_copy(self):
        self.add_polyform_crate(notice=False, license_copy=False)
        self.assert_error("Required Notice")
        self.assert_error("LICENSE.md")

    def test_open_crate_may_not_depend_on_polyform_crate(self):
        self.add_polyform_crate()
        self.edit("crates/septuvo-policy/Cargo.toml", "[dependencies]\n",
                  '[dependencies]\nseptuvo-vpn-core = { path = "../septuvo-vpn-core" }\n')
        self.assert_error("Apache-2.0 crate depends on PolyForm crate")

    def test_polyform_crate_may_depend_on_open_crate(self):
        self.add_polyform_crate(POLYFORM_CRATE + '\n[dependencies]\nseptuvo-policy = { path = "../septuvo-policy" }\n')
        self.assertEqual(self.errors(), [])

    def test_missing_brand_notice_and_licensing_page_fail(self):
        (self.root / "brand/NOTICE.md").unlink()
        self.assert_error("brand rights notice")
        (self.root / "LICENSING.md").unlink()
        self.assert_error("LICENSING.md")

    def test_licensing_page_must_name_every_crate(self):
        self.edit("LICENSING.md", "crates/septuvo-cli", "the CLI crate")
        self.assert_error("does not mention crates/septuvo-cli")


if __name__ == "__main__":
    unittest.main()
