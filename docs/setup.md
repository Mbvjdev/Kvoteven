# Setup and troubleshooting

The 0.4.0 app is a Tauri 2 desktop app. The Swift 0.3.0 menu-bar build is legacy — see the
bottom of this page.

## Install from a package (not yet published)

When v0.4.0 is released, installers will be on
[GitHub Releases](https://github.com/Mbvjdev/Kvoteven/releases) with SHA-256 checksums.
Until then, build from source below. Because the packages are not distribution-signed,
expect Gatekeeper (macOS), SmartScreen (Windows) or an unsigned-package warning (Linux).

## Build from source

Requirements:

- [Node.js 22 LTS](https://nodejs.org/) and [Rust](https://www.rust-lang.org/tools/install)
  (CI pins Rust 1.98.x). The shipped app has no Node/Python/Rust runtime requirement.
- macOS 13+ for the `.dmg`; Ubuntu 22.04+ for the `.deb`; Windows 10/11 for the `.exe`.

```sh
cd desktop
npm ci --ignore-scripts
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

Then build the installer for your platform:

```sh
# macOS (ad-hoc signed; not notarized)
npm run tauri -- build --bundles app,dmg

# Linux (Ubuntu 22.04+ .deb)
npm run tauri -- build --bundles deb

# Windows (per-user NSIS installer; unsigned)
npm run tauri -- build --bundles nsis
```

### Linux prerequisites

Install the Tauri system libraries first (Debian/Ubuntu names):

```sh
sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev libdbus-1-dev libsecret-1-dev
```

The packaged `.deb` depends on the matching runtime libraries
(`libwebkit2gtk-4.1-0`, `libayatana-appindicator3-1`, `libsecret-1-0`).

## Try without connecting anything

Open Settings and enable **Demo mode**, or launch the binary with `--demo`. The panel shows
**Demo · sample data** with fixed fixtures and never reads credentials or calls a provider.
`--check --demo` is rejected, so demo data can never be mistaken for a live check.

## DeepSeek

Enter your API key in Settings (a masked field). It is stored in the OS keyring and never
written to disk in plaintext:

- macOS → Keychain
- Windows → Credential Manager
- Linux → Secret Service

No Hermes, Node, Python or Rust is needed at runtime for DeepSeek. If the keyring is
unavailable, the save is refused rather than falling back to a plaintext file. The balance
is account-wide, not limited to calls made by Kvoteven.

## Codex

Choose a read-only source in Settings (**Auto / Codex CLI / Hermes**):

- **Codex CLI** — install the official CLI and log in in a terminal (`codex login`), using
  the OS-specific versions from the [official docs](https://developers.openai.com/codex/).
  Kvoteven talks to its `app-server` protocol and does not perform login itself.
- **Hermes** — reuse your existing **default** Hermes profile's read-only usage output
  (`hermes --profile default usage --provider openai-codex --json`). No OAuth or tokens are
  copied out of Hermes.

*Auto* prefers the Codex CLI when present, then Hermes, and does not silently switch after
an error. Missing weekly data means the weekly meter cannot be shown; Kvoteven will not guess.

## App controls

Open the panel to switch provider, language or Codex source, refresh, toggle demo, or quit.
The selected provider refreshes every five minutes; manual refreshes have a 15-second minimum
gap. Unknown/failed/stale readings are shown as a dash; data older than ten minutes is stale.
Codex values also expire at their reported reset time.

Closing the window hides it to the tray (menu bar on macOS); **Quit** exits. On Linux, if no
system tray is available (for example some GNOME setups without an AppIndicator extension),
the app relies on single-instance reopen — relaunching Kvoteven re-focuses the existing
window instead of starting a second copy.

**System** language means Danish when the system language is Danish, English otherwise.
You can force English or Danish without changing the OS.

## When a provider is unavailable

- **DeepSeek**: check the key in Settings and the network. The app shows a sanitized error,
  never a raw HTTP body or header.
- **Codex**: check `codex` is installed, logged in, and on `PATH`, or that the Hermes default
  profile is set up. For a custom Hermes executable, launch with `KVOTEVEN_HERMES` set to its
  absolute path — that runs it with your permissions, so use only a path you trust.

## Uninstalling

Remove the app like any other application (drag to Trash on macOS, the package manager or
Apps & features on Windows/Linux). Uninstalling does **not** delete your DeepSeek key from
the OS keyring. **Delete the key in Settings before uninstalling** if you want it gone.

## Distribution limitations

See [distribution](docs/distribution.md) for the full signing and verification matrix.
In short: macOS is ad-hoc signed (not Developer ID, not notarized), Windows
is unsigned, Linux is unsigned with published SHA-256 checksums. There is no automatic
updater and no autostart registration.

## Legacy Swift 0.3.0 app

The original macOS menu-bar app builds from `Sources/` with `python3 scripts/build_app.py`.
It requires macOS 13+, Swift 5.9+ and Python 3, and reads live accounts through Hermes. It
is preserved for reference only and is not the primary distribution.
