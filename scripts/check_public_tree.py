"""Release guard for the exact Git index; never prints matched secret values."""
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath


def inspect_payload(path, data):
    p = PurePosixPath(path)
    allowed_roots = {"Sources", "Tests", "scripts", "docs", ".github"}
    allowed_files = {"Package.swift", "README.md", "README.da.md", "LICENSE",
                     "SECURITY.md", "CONTRIBUTING.md", "CHANGELOG.md", ".gitignore", ".gitattributes"}
    if p.parts[0] not in allowed_roots and path not in allowed_files:
        return ["file is outside the public source allowlist"]
    if any(part.startswith(".env") or part in {".hermes", ".codex", "auth.json", "credentials.json"}
           or "credentials" in part.lower() for part in p.parts):
        return ["private configuration filename"]
    if p.suffix.lower() in {".pem", ".key", ".p12", ".pfx", ".db", ".sqlite", ".log"}:
        return ["private material or local state"]
    patterns = {
        "possible API credential": rb"(?:sk-[A-Za-z0-9_-]{20,}|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,})",
        "private key material": rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----",
        "personal absolute path": rb"/(?:Users|home)/[A-Za-z0-9_.-]+/",
    }
    return [label for label, pattern in patterns.items() if re.search(pattern, data)]


def main():
    root = Path(__file__).resolve().parents[1]
    entries = subprocess.check_output(["git", "ls-files", "--stage", "-z"], cwd=root).split(b"\0")
    findings = []
    count = 0
    for entry in filter(None, entries):
        metadata, raw_path = entry.split(b"\t", 1)
        mode, object_id, stage = metadata.split()
        path = raw_path.decode("utf-8")
        count += 1
        if mode not in (b"100644", b"100755") or stage != b"0":
            findings.append((path, "symlink, submodule, or unresolved merge is not publishable"))
            continue
        data = subprocess.check_output(["git", "cat-file", "blob", object_id.decode()], cwd=root)
        findings.extend((path, reason) for reason in inspect_payload(path, data))
    if not count:
        print("FAIL: no staged/tracked source files to audit")
        return 1
    for path, reason in findings:
        print(f"BLOCKED: {path}: {reason}")
    print(f"Public tree: {count} files, {len(findings)} findings. Values are never printed.")
    return int(bool(findings))


if __name__ == "__main__":
    sys.exit(main())
