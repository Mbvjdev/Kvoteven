import hashlib
import json
from pathlib import Path
import tempfile
import subprocess
import unittest
from collect_desktop import collect

class CollectDesktopTests(unittest.TestCase):
    def test_only_native_verified_packages_get_hashed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "desktop/src-tauri/target/release/bundle/deb/Kvoteven_test.deb"
            package.parent.mkdir(parents=True)
            package.write_bytes(b"synthetic-package-test-only")
            (root / "desktop/package.json").write_text(json.dumps({"version": "0.4.0"}))
            (root / "verification").mkdir()
            report = root / "verification/native-linux.json"
            report.write_text(json.dumps({"version":"0.4.0","platform":"linux","native_gui":False,"ok":True,"mode":"offline_demo"}))
            with self.assertRaises(ValueError):
                collect(root, "linux")
            report.write_text(json.dumps({"version":"0.4.0","platform":"linux","native_gui":True,"ok":True,"mode":"offline_demo"}))
            # A generic or stale GUI result must never authorize an unrelated package.
            with self.assertRaises(ValueError):
                collect(root, "linux")
            evidence = json.loads(report.read_text())
            evidence["package"] = {"file": package.name, "sha256": hashlib.sha256(package.read_bytes()).hexdigest()}
            report.write_text(json.dumps(evidence))
            (root / ".gitignore").write_text("verification/\ndist/\ndesktop/src-tauri/target/\n")
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, text=True).strip()
            git("init", "--quiet")
            git("add", ".gitignore", "desktop/package.json")
            git("-c", "user.name=Synthetic Test", "-c", "user.email=fixture@example.invalid",
                "commit", "--quiet", "-m", "Synthetic test fixture")
            revision = git("rev-parse", "HEAD")
            manifest = collect(root, "linux")
            self.assertEqual(manifest["commit"], revision)
            # Never label a package with a source commit that omits local changes.
            source = root / "desktop/new-source.txt"
            source.write_text("synthetic source\n")
            with self.assertRaisesRegex(ValueError, "clean committed source"):
                collect(root, "linux")
            git("add", "desktop/new-source.txt")
            with self.assertRaisesRegex(ValueError, "clean committed source"):
                collect(root, "linux")
            git("-c", "user.name=Synthetic Test", "-c", "user.email=fixture@example.invalid",
                "commit", "--quiet", "-m", "Add synthetic source")
            source.write_text("changed synthetic source\n")
            with self.assertRaisesRegex(ValueError, "clean committed source"):
                collect(root, "linux")
            git("restore", "desktop/new-source.txt")
            manifest = collect(root, "linux")
            self.assertEqual(len(manifest["artifacts"]), 1)
            self.assertEqual(manifest["artifacts"][0]["sha256"], hashlib.sha256(package.read_bytes()).hexdigest())
            self.assertTrue((root / "dist/packages/SHA256SUMS-linux.txt").is_file())
            self.assertNotIn(str(root), json.dumps(manifest))
            package.write_bytes(b"different-synthetic-package")
            with self.assertRaises(ValueError):
                collect(root, "linux")

if __name__ == "__main__":
    unittest.main()
