# Changelog

## 0.4.0-beta.1 — public cross-platform beta

A new Tauri 2 / Rust / vanilla TypeScript desktop app that replaces the SwiftUI UI
platform dependency while keeping the working Swift 0.3.0 build as a legacy preview.

- macOS, Linux and Windows packages (universal `.dmg`, `.deb`, NSIS `.exe`) as build targets.
- DeepSeek works standalone: API key stored in the OS keyring (Keychain / Credential
  Manager / Secret Service), no Hermes or script runtimes required.
- Codex via the official Codex CLI `app-server` protocol or an existing Hermes default
  profile's read-only usage output. No OAuth or tokens copied from Hermes.
- Bilingual English/Danish UI, light/dark, and the original pixel pet.
- Explicit offline demo (in-app toggle or `--demo`) with fixed synthetic fixtures;
  `--check --demo` is rejected so demo data can never pass a live check.
- Sanitized `--check` CLI and a `--demo --smoke-test` native WebView/IPC proof.

**Public beta, not production approval.** The Apple Silicon DMG has passed local native
CLI/WebView/IPC tests and binary privacy checks. Other platform tests are pending at
publication. Packages are not distribution-signed. See [known limitations](docs/beta-0.4.0.md).

## 0.3.0 — first public preview

- Codex weekly quota and DeepSeek API balance in one native macOS menu-bar app.
- A pixel pet with quota/balance-aware moods.
- Selectable system, English and Danish language, including number/date formatting.
- Explicit offline demo mode for public screenshots and account-free testing.
- Read-only adapters using existing Hermes credentials; no new credential store.
- Public-source guard, secret scanning and account-free macOS CI.

This is a developer preview. Hermes is required for live data, and the app is not
Apple-notarized. No historical spending report or combined ChatGPT-wide meter is claimed.
