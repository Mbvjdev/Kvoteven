import json
import unittest
from check_desktop import smoke_proof

class DesktopChecksTests(unittest.TestCase):
    def test_requires_real_ipc_render_proof_not_just_exit_zero(self):
        proof = {"native_smoke": True, "providers": ["deepseek", "openai-codex"], "languages": ["en", "da"], "source": "demo_synthetic"}
        self.assertTrue(smoke_proof(json.dumps(proof)))
        self.assertFalse(smoke_proof("app started"))
        self.assertFalse(smoke_proof(json.dumps({**proof, "native_smoke": False})))
        self.assertFalse(smoke_proof(json.dumps({**proof, "providers": ["deepseek"]})))
        self.assertFalse(smoke_proof(json.dumps({**proof, "source": "live"})))

if __name__ == "__main__":
    unittest.main()
