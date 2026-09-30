import unittest
import subprocess
from pathlib import Path
from check_public_tree import inspect_payload


class PublicTreeTests(unittest.TestCase):
    def test_rejects_private_runtime_and_verification_files(self):
        for path in (".env", "docs/.env.local", "auth.json", "verification/live.png",
                     "build/Kvoteven.app/Contents/MacOS/Kvoteven", "Sources/private.key"):
            with self.subTest(path=path):
                self.assertTrue(inspect_payload(path, b"not-a-secret"))

    def test_detects_keys_and_private_paths_without_echoing_the_match(self):
        for value in (b"sk-" + b"a" * 40, b"ghp_" + b"b" * 40,
                      b"-----BEGIN " + b"PRIVATE KEY-----",
                      b"/Users/" + b"private-owner/Projects/", b"/home/" + b"private-owner/.hermes/",
                      b"C:\\Users\\" + b"private-owner\\Projects\\"):
            findings = inspect_payload("Sources/Example.swift", value)
            self.assertTrue(findings)
            self.assertNotIn(value.decode(), str(findings))

    def test_desktop_sources_are_allowed_but_generated_state_is_not(self):
        for path in ("desktop/package.json", "desktop/package-lock.json", "desktop/src/main.ts",
                     "desktop/src-tauri/Cargo.lock", "desktop/src-tauri/src/vault.rs",
                     "desktop/src-tauri/icons/icon.ico", "desktop/.cargo/config.toml"):
            with self.subTest(path=path):
                self.assertEqual(inspect_payload(path, b"source-only"), [])
        for path in ("desktop/node_modules/pkg/index.js", "desktop/src-tauri/target/release/app",
                     "desktop/test-results/screenshot.png", "desktop/src-tauri/gen/schemas/acl.json",
                     "docs/assets/live-account.png", "desktop/src-tauri/icons/live-account.png"):
            with self.subTest(path=path):
                self.assertTrue(inspect_payload(path, b"private local state"))

    def test_native_generated_schemas_are_ignored_at_the_real_repo_path(self):
        result = subprocess.run(["git", "check-ignore", "--no-index", "desktop/src-tauri/gen/schemas/acl-manifests.json"], cwd=Path(__file__).resolve().parents[1], capture_output=True)
        self.assertEqual(result.returncode, 0)

    def test_normal_sources_and_obvious_dummy_fixture_are_allowed(self):
        self.assertEqual(inspect_payload("README.md", b"DEEPSEEK_API_KEY stays local."), [])
        self.assertEqual(inspect_payload("Tests/Example.swift", b"test-only-key"), [])


if __name__ == "__main__":
    unittest.main()
