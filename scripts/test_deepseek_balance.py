import unittest
from datetime import datetime, timezone
from deepseek_balance import fetch_balance

class BalanceBridgeTests(unittest.TestCase):
    def test_rejects_missing_key_without_network_and_sanitizes_failure(self):
        calls = []
        def broken_get(*args, **kwargs):
            calls.append(args)
            raise RuntimeError("secret-marker-in-request")
        with self.assertRaisesRegex(ValueError, "credential"):
            fetch_balance(None, broken_get)
        self.assertEqual(calls, [])
        with self.assertRaises(ValueError) as caught:
            fetch_balance("test-only-key", broken_get)
        self.assertNotIn("secret-marker", str(caught.exception))

    def test_http_errors_or_empty_payload_cannot_be_a_success(self):
        class Response:
            status_code = 401
            def json(self):
                return {"is_available": True, "balance_infos": []}
        with self.assertRaises(ValueError):
            fetch_balance("test-only-key", lambda *a, **k: Response())
        Response.status_code = 200
        with self.assertRaises(ValueError):
            fetch_balance("test-only-key", lambda *a, **k: Response())

    def test_calls_only_official_read_only_balance_endpoint(self):
        calls = []
        class Response:
            status_code = 200
            def json(self):
                return {"is_available": True, "balance_infos": [{"currency": "USD", "total_balance": "42.37",
                        "granted_balance": "0.00", "topped_up_balance": "42.37"}], "unneeded": "discard"}
        def get(url, **kwargs):
            calls.append((url, kwargs))
            return Response()
        result = fetch_balance("test-only-key", get, now=datetime(2026, 9, 29, tzinfo=timezone.utc))
        self.assertEqual(result["balance_infos"][0]["total_balance"], "42.37")
        self.assertEqual(result["source"], "deepseek_balance_api")
        self.assertNotIn("unneeded", result)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][0], "https://api.deepseek.com/user/balance")
        self.assertEqual(calls[0][1]["headers"]["Authorization"], "Bearer test-only-key")
        self.assertFalse(calls[0][1]["follow_redirects"])
        self.assertFalse(calls[0][1].get("trust_env", True))
        self.assertEqual(calls[0][1]["timeout"], 15)

if __name__ == "__main__":
    unittest.main()
