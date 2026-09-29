# Security and privacy

## Data flow

| Component | Reads | Sends | Persists |
|---|---|---|---|
| Swift menu-bar app | Sanitized quota / balance JSON | Fixed local subprocess arguments | Provider and language preferences |
| Codex adapter | Hermes `usage` output for the default profile | Hermes performs the provider's authenticated quota request | No extra credential store in Kvoteven |
| DeepSeek helper | `DEEPSEEK_API_KEY` through Hermes' resolver | Authenticated HTTPS GET to `https://api.deepseek.com/user/balance` | Nothing |
| Demo mode | Explicit synthetic fixtures | No requests; no credential reads | Nothing; preferences are unchanged |

The DeepSeek key exists transiently in the helper's memory and in the authorization
header sent to DeepSeek. The helper ignores environment-configured proxies and TLS
overrides (`trust_env=False`) and uses normal certificate validation. “Local” does **not** mean the provider never receives its own
credential. Redirects are disabled, and errors shown by the app do not include raw
exceptions, HTTP bodies or authorization headers.

Kvoteven has no backend, telemetry, analytics or advertising SDK. It makes no model
requests, credit purchases or automatic top-ups. It does not reset or bypass usage limits.
Hermes itself may refresh OAuth credentials through its normal authentication flow.
Hermes' behavior and credential storage remain outside this project's control.

## Local storage and permissions

- Existing credentials remain managed by Hermes. Kvoteven does not copy them into its bundle.
- Snapshots and the session balance baseline are held in memory, not in a usage database.
- Only provider and language choices are stored in macOS UserDefaults.
- No LaunchAgent, cron job or Login Item is installed.
- This preview is not sandboxed or Apple-notarized. It runs subprocesses with your user
  permissions, so only use a trusted Hermes executable and trusted local helper.
- `KVOTEVEN_HERMES` selects a local executable, not a remote URL. It is a trust decision.
- Demo mode is explicit and never a fallback for failed live authentication.

## What must never be published

Do not commit `.env` files, `auth.json`, cookies, tokens, keys, provider responses,
local logs, screenshots of private account usage or compiled local debug artifacts.
`.gitignore` excludes local build and verification directories. The public-tree guard
checks the exact Git index and CI scans Git history with Gitleaks.

Only deliberately synthetic demo images belong in `docs/assets/`.
Live `--check`, `--ui-smoke` and `--render` output contains account metrics. Treat those
outputs as private even though they do not intentionally print credentials.

Scanners reduce risk; they cannot prove that arbitrary images or newly added code are
safe. Review every staged file and every release asset before publication.

## Reporting a vulnerability

Use GitHub's **Report a vulnerability** option on this repository's Security tab.
If private reporting is unavailable, open an issue saying only that you need a private
contact channel. Do not include a key, exploit containing private data or account dump.

If you accidentally publish a credential, revoke/rotate it with its provider first.
Deleting a file or commit is not enough: public copies and caches may remain.

Only the latest preview is maintained. This small project offers no security-response SLA.
