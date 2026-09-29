import unittest
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
                      b"/Users/" + b"private-owner/Projects/", b"/home/" + b"private-owner/.hermes/"):
            findings = inspect_payload("Sources/Example.swift", value)
            self.assertTrue(findings)
            self.assertNotIn(value.decode(), str(findings))

    def test_normal_sources_and_obvious_dummy_fixture_are_allowed(self):
        self.assertEqual(inspect_payload("README.md", b"DEEPSEEK_API_KEY stays local."), [])
        self.assertEqual(inspect_payload("Tests/Example.swift", b"test-only-key"), [])


if __name__ == "__main__":
    unittest.main()
