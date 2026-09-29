"""Live E2E acceptance: real DeepSeek balance, never fixtures."""
import json
import subprocess
from decimal import Decimal
from pathlib import Path

binary = Path(__file__).resolve().parents[1] / "build/Kvoteven.app/Contents/MacOS/Kvoteven"
result = subprocess.run([str(binary), "--check", "--provider", "deepseek"],
                        capture_output=True, text=True, timeout=40)
assert result.returncode == 0, "Could not read live DeepSeek balance"
data = json.loads(result.stdout)
assert data["provider"] == "deepseek", "App has no DeepSeek adapter yet"
assert data["source"] == "deepseek_balance_api"
assert data["currency"] in ("USD", "CNY")
assert Decimal(data["balance"]).is_finite()
assert "weekly_remaining" not in data, "An API balance is not a weekly quota"
assert data["mood"] in ("happy", "steady", "sleepy", "asleep")
print(json.dumps({"result": "PASS", **data}, ensure_ascii=False, indent=2))
