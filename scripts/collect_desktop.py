"""Collect distributable candidates only after native packaged-app verification."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys


def collect(root, platform):
    root = Path(root)
    naming = {"darwin": ("macos", "**/bundle/dmg/*.dmg"), "linux": ("linux", "**/bundle/deb/*.deb"),
              "win32": ("windows", "**/bundle/nsis/*setup.exe")}
    if platform not in naming:
        raise ValueError("Unsupported build platform")
    label, pattern = naming[platform]
    version = json.loads((root / "desktop/package.json").read_text())["version"]
    report = json.loads((root / f"verification/native-{label}.json").read_text())
    if not (report.get("ok") is True and report.get("native_gui") is True
            and report.get("version") == version and report.get("platform") == platform
            and report.get("mode") == "offline_demo"):
        raise ValueError("A passing native packaged-app test is required")
    packages = sorted((root / "desktop/src-tauri/target").glob(pattern))
    if len(packages) != 1:
        raise ValueError("Expected exactly one distribution package")
    package = packages[0]
    evidence = report.get("package")
    if not isinstance(evidence, dict) or evidence.get("file") != package.name or evidence.get("sha256") != hashlib.sha256(package.read_bytes()).hexdigest():
        raise ValueError("Native test evidence does not match this exact package")
    # A local dirty tree cannot truthfully identify its build as HEAD.
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, capture_output=True, text=True)
    state = subprocess.run(["git", "status", "--porcelain", "--untracked-files=normal"], cwd=root, capture_output=True, text=True)
    if revision.returncode != 0 or state.returncode != 0 or state.stdout.strip():
        raise ValueError("Distribution collection requires clean committed source")
    output = root / "dist/packages"
    output.mkdir(parents=True, exist_ok=True)
    artifacts = []
    for package in packages:
        digest = hashlib.sha256(package.read_bytes()).hexdigest()
        shutil.copy2(package, output / package.name)
        artifacts.append({"file": package.name, "sha256": digest, "bytes": package.stat().st_size})
    manifest = {"version": version, "platform": platform, "commit": revision.stdout.strip(),
                "release_channel": "candidate", "signing": "ad-hoc-not-notarized" if platform == "darwin" else "unsigned",
                "native_gui_verified": True, "artifacts": artifacts}
    (output / f"manifest-{label}.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (output / f"SHA256SUMS-{label}.txt").write_text("".join(f"{a['sha256']}  {a['file']}\n" for a in artifacts))
    return manifest


if __name__ == "__main__":
    print(json.dumps(collect(Path(__file__).resolve().parents[1], sys.platform), indent=2))
