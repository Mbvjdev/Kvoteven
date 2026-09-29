"""Offline packaged-app acceptance. Never runs a live account check."""
import os
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
binary = root / "build/Kvoteven.app/Contents/MacOS/Kvoteven"
out = root / "verification"
out.mkdir(exist_ok=True)
# Even if an accidental live path is reached, the selected local reader cannot run.
env = dict(os.environ, KVOTEVEN_HERMES="/usr/bin/false")
for args in (["--check", "--demo"], ["--render", str(out / "bad.png"), "--demo", "--nonsense"],
             ["--language", "fr", "--demo"], ["--render"]):
    p = subprocess.run([str(binary), *args], env=env, capture_output=True, timeout=10)
    assert p.returncode == 2, "Invalid CLI combination was not rejected"
count = 0
for provider in ("openai-codex", "deepseek"):
    for language in ("en", "da"):
        for dark in (False, True):
            path = out / f"demo-{provider}-{language}-{'dark' if dark else 'light'}.png"
            args = [str(binary), "--render", str(path), "--demo", "--provider", provider, "--language", language]
            if dark:
                args.append("--dark")
            p = subprocess.run(args, env=env, capture_output=True, timeout=15)
            assert p.returncode == 0, "Demo render failed"
            assert path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n"), "Invalid PNG"
            count += 1
print(f"PASS: {count} offline renders; both providers/languages/appearances; invalid CLI rejected")
