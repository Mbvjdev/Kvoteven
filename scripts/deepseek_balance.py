"""Read-only DeepSeek balance bridge. Runs inside Hermes' managed Python runtime.

No credential is returned, persisted, logged, or placed in argv. Only the fixed
DeepSeek origin receives the key. This script never makes a model request.
"""
import json
from datetime import datetime, timezone

ENDPOINT = "https://api.deepseek.com/user/balance"


def fetch_balance(key, get, now=None):
    if not isinstance(key, str) or not key.strip():
        raise ValueError("Missing DeepSeek credential")
    try:
        response = get(ENDPOINT, headers={"Authorization": "Bearer " + key},
                       timeout=15, follow_redirects=False, trust_env=False)
        if response.status_code != 200:
            raise ValueError("Balance endpoint rejected the request")
        payload = response.json()
        if (not isinstance(payload, dict) or type(payload.get("is_available")) is not bool
                or not isinstance(payload.get("balance_infos"), list) or not payload["balance_infos"]):
            raise ValueError("Missing provider balance")
    except Exception:
        raise ValueError("DeepSeek balance request failed") from None
    return {
        "provider": "deepseek",
        "source": "deepseek_balance_api",
        "fetched_at": (now or datetime.now(timezone.utc)).isoformat(timespec="seconds"),
        "is_available": payload["is_available"],
        "balance_infos": [{field: item[field] for field in
            ("currency", "total_balance", "granted_balance", "topped_up_balance")}
            for item in payload["balance_infos"]],
    }


def main():
    try:
        import httpx
        from hermes_cli.config import get_env_value_prefer_dotenv
        key = get_env_value_prefer_dotenv("DEEPSEEK_API_KEY")
        result = fetch_balance(key, httpx.get)
    except Exception:
        # Do not echo provider error bodies, exception strings, request objects, or keys.
        print(json.dumps({"error": "DeepSeek saldo kunne ikke hentes. Kontrollér nøgle og forbindelse."}))
        return 1
    print(json.dumps(result, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
