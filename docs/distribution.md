# Distribution and signing

This page states exactly what the 0.4.0 candidate packages are and are not. Read it before
sharing or shipping a build.

## Package matrix

| Platform | Package | Target OS | Signing today | Verification today |
|---|---|---|---|---|
| macOS | Universal `.dmg` (Apple Silicon + Intel) | macOS 13+ | Ad-hoc — **not** Developer ID, **not** notarized | Build + offline native smoke, locally/CI target |
| Linux | `.deb` (x86_64) | Ubuntu 22.04+ | Unsigned | Build + offline native smoke (Xvfb), CI target |
| Windows | NSIS installer (x64, per-user) | Windows 10 / 11 + WebView2 | Unsigned | Build + offline native smoke, CI target |

**The first public beta is [v0.4.0-beta.1](beta-0.4.0.md).** Its Apple Silicon Mac DMG
has passed local native tests. The universal Mac, Linux and Windows entries above remain
CI build targets until their jobs pass. Every package is a prerelease candidate and carries
a `candidate` release channel in its manifest. See the release page for available files.

## Signing status, honestly

- **macOS**: builds are ad-hoc signed. They are **not** Developer ID
  signed and **not** notarized, so Gatekeeper will warn on first launch after download.
  This is a developer preview, not an App Store or notarized distribution.
- **Windows**: installers are **unsigned**. SmartScreen may show an "unknown publisher"
  warning. A Windows code-signing certificate is not available.
- **Linux**: packages are **unsigned**; there is no signed apt repository. SHA-256
  checksums are published alongside packages so you can verify downloads.

Do not describe these packages as "production-ready", "signed", "notarized" or
"guaranteed to work on OS X". Stating the target OS minimums above is not a compatibility
guarantee — compatibility is only proven by the real-machine tests in the release checklist.

## What is and is not implemented

Implemented:

- Manual install via the packages above or a source build.
- Per-user preferences (provider, language, Codex source) in local app settings.
- The DeepSeek key in the OS keyring, and the ability to delete it from Settings.

Not implemented:

- **No autostart**: the app does not register itself to launch at login.
- **No automatic updater**: you check for new versions manually and verify checksums from
  the release page.
- **No key migration or recovery**: the DeepSeek key exists only in the OS keyring.

## Uninstalling

Remove the app as you would any application. Uninstalling does **not** delete your DeepSeek
key from the OS keyring — the key is stored outside the app bundle. **Delete the key in
Settings before uninstalling** if you want it removed. There is no automatic key deletion.

## Verifying a download

Each release package is accompanied by a `SHA256SUMS-<platform>.txt`. Verify before
installing, for example on Linux/macOS:

```sh
sha256sum -c SHA256SUMS-linux.txt   # or shasum -a 256 -c on macOS
```

The app can be built from source with locked dependency versions in
`desktop/package-lock.json` and `desktop/src-tauri/Cargo.lock`. Byte-for-byte
reproducibility of signed installers has not been established.

The manifest binds the download's SHA-256 to its native test report and records the
clean source checkout's commit. This is **not** independent proof that arbitrary package
bytes were built from that commit. Build provenance comes from the linked CI run's
checkout → locked build → installed-package test → upload sequence; local builds must
record and preserve the same source-to-build relationship.

## Production release requirements

A package may only be called production-ready after all of the following, none of which can
be faked:

1. Independent, real-machine verification of the native Windows, Linux and macOS packages
   (install, launch, and the native WebView/IPC smoke proof on each).
2. Independent security and logic review of the complete diff.
3. Real signing credentials: Apple Developer ID + notarization (macOS), an Authenticode
   certificate (Windows), and a signed repository or distro acceptance (Linux).
4. An official GitHub Release whose tag, commit SHA, checksums and artifacts are read back
   and verified against the audited source.

Until those hold, this project ships prerelease candidates only, and the docs say so.
