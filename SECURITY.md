# Security and privacy

Kvoteven 0.4.0 is a Tauri 2 / Rust desktop app. This document describes the desktop app
first; the legacy Swift 0.3.0 build is covered at the end and is being replaced.

## Data flow

| Component | Reads | Sends | Persists |
|---|---|---|---|
| Desktop app (Tauri/Rust) | Sanitized quota / balance results | Fixed, allow-listed local subprocess arguments | Provider, language and Codex-source preferences only |
| DeepSeek adapter | Your API key from the **OS keyring** | Authenticated HTTPS GET to `https://api.deepseek.com/user/balance` | Nothing in the app |
| Codex adapter | `codex app-server` protocol over stdio | The official CLI performs the authenticated request | Nothing in the app |
| Hermes adapter | `hermes --profile default usage --provider openai-codex --json` output | Hermes performs the authenticated quota request | Nothing in the app |
| Demo mode | Explicit synthetic fixtures (42.50 USD / 68%) | No requests; no credential reads | Nothing; preferences are unchanged |

## Credentials

- The DeepSeek key is entered in a **masked** field in Settings and stored in the
  **OS keyring** — Keychain on macOS, Credential Manager on Windows, Secret Service on
  Linux — under a fixed service/account pair. There is **no plaintext fallback**: if the
  keyring is unavailable, the save is refused with a sanitized error.
- The key exists transiently in memory and in the authorization header sent to DeepSeek.
  It is never written to process arguments, environment variables, app preferences,
  screenshots, logs or the repository.
- The HTTP client ignores environment-configured proxies and TLS overrides, uses normal
  certificate validation, and disables redirects. “Local” does **not** mean the provider
  never receives its own credential.
- Codex is read through the official CLI's `app-server` protocol or Hermes' read-only
  `usage` command. Kvoteven stores no Codex credential, performs no Codex login, and copies
  no OAuth tokens out of Hermes.
- Errors shown by the app are fixed sanitized codes; raw response bodies, process output,
  auth headers and keyring errors are never surfaced.

Kvoteven has no backend, telemetry, analytics or advertising SDK. It makes no model
requests, credit purchases or automatic top-ups. It does not reset or bypass usage limits.

## Local storage and permissions

- Snapshots and the session balance baseline are held in memory, not in a usage database.
- Only **provider, language and Codex source** choices are persisted (local app settings).
  No key, login, quota value or account data is ever persisted by the app.
- No autostart registration, LaunchAgent, cron job or startup item is installed. There is
  no automatic updater; updates are manual with published checksums.
- This candidate is **not** distribution-signed (see [distribution](docs/distribution.md)).
  It runs with your user permissions, so only run packages you can verify and subprocesses
  (Codex CLI / Hermes) you trust.
- `KVOTEVEN_HERMES` selects a local Hermes executable, not a remote URL. It is a trust decision.
- `KVOTEVEN_CODEX` selects a local **native** Codex executable by absolute path, never a
  remote URL or a shell shim. It is a trust decision: Kvoteven runs that exact file with
  your permissions. Codex discovery otherwise searches only the fixed install locations
  documented in [setup](docs/setup.md) plus the `PATH`; it never runs a shell or a
  `.cmd`/`.bat` shim, never reads Codex auth, and never does an arbitrary recursive scan
  of your home directory.
- Demo mode is explicit and never a fallback for failed live authentication.

## What must never be published

Do not commit `.env` files, `auth.json`, cookies, tokens, keys, provider responses,
local logs, screenshots of private account usage or compiled local debug artifacts.
`.gitignore` excludes build, `node_modules` and `target` directories. The public-tree guard
checks the exact Git index and CI scans Git history with Gitleaks.

Only deliberately synthetic demo images belong in public docs. Live `--check` and
`--smoke-test` output contains account metrics (sanitized, but still private). Treat those
outputs as private even though they do not print credentials.

Scanners reduce risk; they cannot prove that arbitrary images or newly added code are
safe. Review every staged file and every release asset before publication.

## Reporting a vulnerability

Use GitHub's **Report a vulnerability** option on this repository's Security tab.
If private reporting is unavailable, open an issue saying only that you need a private
contact channel. Do not include a key, exploit containing private data or account dump.

If you accidentally publish a credential, revoke/rotate it with its provider first.
Deleting a file or commit is not enough: public copies and caches may remain.

Only the latest candidate is maintained. This small project offers no security-response SLA.

## Legacy Swift 0.3.0 app

The original macOS menu-bar app read DeepSeek via Hermes' credential resolver and Codex via
Hermes' usage output, storing only provider/language in UserDefaults. It remains in the
repository under `Sources/` but is no longer the primary distribution; its behavior and
storage model are superseded by the desktop app above.
