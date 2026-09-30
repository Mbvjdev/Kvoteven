"""Exercise an actual packaged Kvoteven executable, without credentials.

The GUI check starts its native WebView and requires the renderer to read both
providers through Rust IPC, render both languages, and acknowledge the proof.
A process merely exiting zero is not a passing GUI check.
"""
import argparse
import hashlib
import json
from check_public_tree import inspect_payload
from pathlib import Path
import subprocess
import sys


def smoke_proof(output):
    for line in output.splitlines():
        try:
            value = json.loads(line)
        except (ValueError, TypeError):
            continue
        if not isinstance(value, dict):
            continue
        if (value.get("native_smoke") is True
                and sorted(value.get("providers", [])) == ["deepseek", "openai-codex"]
                and sorted(value.get("languages", [])) == ["da", "en"]
                and value.get("source") == "demo_synthetic"):
            return True
    return False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("--gui", action="store_true", help="Require a real desktop/display and native WebView proof")
    parser.add_argument("--report", type=Path)
    parser.add_argument("--package", type=Path, help="Bind test evidence to this exact installer/package")
    args = parser.parse_args()
    executable = args.executable.resolve(strict=True)
    binary = executable.read_bytes()
    findings = inspect_payload("desktop/native-binary", binary)
    if findings:
        raise RuntimeError("Release binary privacy check failed: " + ", ".join(findings))
    package_proof = None
    if args.package:
        package_proof = {"file": args.package.name, "sha256": hashlib.sha256(args.package.read_bytes()).hexdigest()}
    expected_version = json.loads((Path(__file__).resolve().parents[1] / "desktop/package.json").read_text())["version"]
    cases = [(["--version"], 0), (["--help"], 0), (["--check", "--demo"], 2),
             (["--smoke-test"], 2), (["--unknown"], 2)]
    for flags, expected in cases:
        result = subprocess.run([str(executable), *flags], capture_output=True, text=True, timeout=15)
        if result.returncode != expected:
            raise RuntimeError(f"CLI exit mismatch for {flags}: {result.returncode}")
        if flags == ["--version"] and result.stdout.strip() != expected_version:
            raise RuntimeError("Packaged executable version does not match manifest")
    if args.gui:
        result = subprocess.run([str(executable), "--demo", "--smoke-test"], capture_output=True, text=True, timeout=65)
        if result.returncode != 0 or not smoke_proof(result.stdout):
            # This mode contains only synthetic data; still keep output bounded.
            print(result.stdout[-2000:], result.stderr[-2000:], file=sys.stderr)
            raise RuntimeError("Packaged native WebView/IPC proof failed")
    report = {"version": expected_version, "platform": sys.platform, "cli_cases": len(cases),
              "native_gui": args.gui, "mode": "offline_demo", "ok": True}
    if package_proof:
        if package_proof["sha256"] != hashlib.sha256(args.package.read_bytes()).hexdigest():
            raise RuntimeError("Distribution package changed during verification")
        report["package"] = package_proof
    report["executable_sha256"] = hashlib.sha256(binary).hexdigest()
    report["binary_privacy_checked"] = True
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
