"""Build a local, ad-hoc-signed macOS app. No installer or login item."""
import plistlib
import shutil
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
subprocess.run(["swift", "build", "-c", "release",
                "-Xswiftc", "-file-prefix-map", "-Xswiftc", f"{Path.home()}=/build-home",
                "-Xswiftc", "-gnone"],
               cwd=root, check=True)
app = root / "build/Kvoteven.app"
contents = app / "Contents"
(contents / "MacOS").mkdir(parents=True, exist_ok=True)
(contents / "Resources").mkdir(exist_ok=True)
shutil.copy2(root / ".build/release/Kvoteven", contents / "MacOS/Kvoteven")
subprocess.run(["/usr/bin/strip", "-S", str(contents / "MacOS/Kvoteven")], check=True)
shutil.copy2(root / "scripts/deepseek_balance.py", contents / "Resources/deepseek_balance.py")
with (contents / "Info.plist").open("wb") as handle:
    plistlib.dump({
        "CFBundleIdentifier": "dk.justservices.kvoteven",
        "CFBundleName": "Kvoteven",
        "CFBundleDisplayName": "Kvoteven",
        "CFBundleExecutable": "Kvoteven",
        "CFBundlePackageType": "APPL",
        "CFBundleVersion": "3",
        "CFBundleShortVersionString": "0.3.0",
        "LSMinimumSystemVersion": "13.0",
        "LSUIElement": True,
        "NSHighResolutionCapable": True,
        "NSPrincipalClass": "NSApplication",
    }, handle)
subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
if "--install" in sys.argv:
    destination = Path.home() / "Applications/Kvoteven.app"
    if destination.exists():
        with (destination / "Contents/Info.plist").open("rb") as handle:
            if plistlib.load(handle).get("CFBundleIdentifier") != "dk.justservices.kvoteven":
                sys.exit("Refusing to overwrite a different app")
    destination.parent.mkdir(exist_ok=True)
    shutil.copytree(app, destination, dirs_exist_ok=True)
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(destination)], check=True)
    print(destination)
else:
    print(app)
