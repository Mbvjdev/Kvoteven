# Kvoteven v0.4.0-beta.1

**Public testing release, not production approval.** The application reports version
`0.4.0`; the Git tag `v0.4.0-beta.1` identifies this exact beta source snapshot.

[Downloads](https://github.com/Mbvjdev/Kvoteven/releases/tag/v0.4.0-beta.1) ·
[Report a bug](https://github.com/Mbvjdev/Kvoteven/issues)

## Available and pending

- **macOS Apple Silicon:** the attached DMG has passed local CLI checks, the actual
  packaged WebView/IPC smoke test for both providers and languages, codesign verification,
  and a binary scan for credentials/private build paths. It is ad-hoc signed, **not
  notarized**. macOS can block an app downloaded from an unidentified developer.
- **macOS Intel/universal, Linux x64 and Windows x64:** native build/test jobs are
  triggered by this tag. Only packages that pass their own platform's automated checks
  are attached by CI. A missing download means that platform has not passed yet.
- Automated smoke tests use explicitly synthetic offline data. They are not proof of
  live account access on every platform. No credentials or account screenshots are shipped.

## Known limitations in this beta

1. **Codex discovery:** a GUI launch with a minimal PATH may miss a global Homebrew/npm
   installation. An explicit absolute `KVOTEVEN_CODEX` path to the native Codex binary is
   supported when launching the app from a terminal. Windows `.cmd`/`.bat` wrappers are
   deliberately rejected. Alternatively, use an existing working Hermes installation.
   Broader native discovery is being fixed for the next beta.
2. **Windows process cleanup:** the current runner terminates the direct helper process,
   not its entire descendant tree. A pip-installed Hermes `.exe` launcher may leave its
   Python subprocess running after a timeout. Native descendant cleanup is being added.
3. **Signing:** no Apple Developer ID/notarization, Windows Authenticode certificate,
   or signed Linux package repository. Windows SmartScreen may warn about an unknown
   publisher. SHA-256 checksums accompany the downloads.
4. No automatic updater, autostart, historical billing report, or credential migration.
5. Native platform CI has not completed at initial publication. Check the actual workflow
   and release assets instead of assuming every advertised build target is available.

## Try it safely

Start with `--demo` to exercise the pet without network calls or credentials. The demo
banner stays visible, values are fictional, and credential entry is hidden. Live DeepSeek
setup stores your own key in the operating system's credential store; do not paste it
into an issue. Codex requires your own already-authenticated official CLI or Hermes.

For bug reports include the OS/version, beta tag, selected provider/source, and sanitized
error code. **Never include API keys, access tokens, account exports, or unredacted logs.**

The Swift 0.3.0 preview remains available separately; this beta does not replace an
existing installation automatically.
