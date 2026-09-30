"""Mount and test the built DMG itself, then always detach it."""
import plistlib
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
images = list((root / "desktop/src-tauri/target").glob("**/bundle/dmg/*.dmg"))
if len(images) != 1:
    raise SystemExit("Expected exactly one packaged DMG")
attached = plistlib.loads(subprocess.check_output(["hdiutil", "attach", "-readonly", "-nobrowse", "-plist", str(images[0])]))
mounts = [Path(e["mount-point"]) for e in attached.get("system-entities", []) if "mount-point" in e]
try:
    if len(mounts) != 1:
        raise RuntimeError("Expected one mounted volume")
    app = mounts[0] / "Kvoteven.app"
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
    subprocess.run([sys.executable, str(root / "scripts/check_desktop.py"), str(app / "Contents/MacOS/kvoteven"), "--gui", "--package", str(images[0]), "--report", str(root / "verification/native-macos.json")], check=True)
finally:
    for mount in mounts:
        subprocess.run(["hdiutil", "detach", str(mount)], check=True)
