"""End-to-end acceptance check. Calls the REAL read-only quota endpoint."""
import json
import subprocess
import sys
from pathlib import Path

binary = Path(__file__).resolve().parents[1] / "build/Kvoteven.app/Contents/MacOS/Kvoteven"
if not binary.is_file():
    sys.exit("FAIL: Kvoteven app executable has not been built")
result = subprocess.run([str(binary), "--check", "--provider", "openai-codex"], capture_output=True, text=True, timeout=35)
assert result.returncode == 0, "Live quota read failed (no synthetic fallback allowed)"
data = json.loads(result.stdout)
assert data["provider"] == "openai-codex"
assert isinstance(data["weekly_remaining"], (float, int))
assert 0 <= data["weekly_remaining"] <= 100
assert data["source"] == "usage_api"
assert data["mood"] in ["happy", "steady", "sleepy", "asleep"]
print(json.dumps({"result": "PASS", **data}, ensure_ascii=False, indent=2))
