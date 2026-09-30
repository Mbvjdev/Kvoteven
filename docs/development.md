# Development and verification

The desktop app lives in `desktop/`. The Swift app under `Sources/` is a legacy build with
its own tests — keep the two test suites separate.

## Offline checks

```sh
cd desktop
npm ci --ignore-scripts
npm test          # Vitest
npm run build     # tsc --noEmit + vite build
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
```

Legacy Swift (preserved):

```sh
swift test
python3 -m unittest discover -s scripts -p 'test_*.py' -v
```

## Command-line surface

The packaged native binary accepts:

```text
kvoteven [--demo [--smoke-test]]
kvoteven --check [--provider deepseek|openai-codex] [--source auto|codex|hermes]
kvoteven --help | --version
```

- `--demo` runs the network-free demo. `--smoke-test` requires `--demo` and, when the
  renderer proves both providers and both languages through real IPC, prints a single
  `{"native_smoke":true,...}` line and exits zero.
- `--check` performs one real read and prints **only sanitized fields** (`ok`, `provider`,
  `source`, `fresh`, `weekly_present`) — never quota values or credentials. `--check --demo`
  is rejected (exit 2) so a fixture can never pass a live check.

`scripts/check_desktop.py` exercises a packaged executable's CLI cases and, with `--gui`,
requires the native WebView/IPC proof. `scripts/check_macos_desktop.py` mounts the built
`.dmg`, verifies its code signature, runs the GUI check, and detaches.

## Public-safe screenshots

Only demo output is suitable for public docs. The demo fixtures are fixed (42.50 USD /
68% remaining) with a visible **Demo** banner and source `demo_synthetic`; no credentials
or network are touched. Browser screenshots are produced by the Playwright suite
(`desktop/tests/screenshots.spec.ts`) into `desktop/verification/`. Copy only reviewed,
explicitly marked demo images to public docs.

## Live checks (private local output)

These require configured accounts and use the real read-only provider paths. They print
account metrics locally and are **not** run in public CI:

```sh
# DeepSeek (key from OS keyring)
kvoteven --check --provider deepseek

# Codex via the official CLI
kvoteven --check --provider openai-codex --source codex

# Codex via Hermes default profile
kvoteven --check --provider openai-codex --source hermes
```

Treat the output as private even though it is sanitized.

## Release checklist

A production release requires, in order:

1. Pass offline tests, lint, browser tests, packaged-app checks and demo renders on all
   three platforms.
2. Independent real-machine tests: install and smoke-test the native Windows, Linux and
   macOS packages (not just CI runners).
3. Independent security/logic review of the complete diff.
4. Signing: a real Apple Developer ID + notarization for macOS, an Authenticode cert for
   Windows, and a signed Linux repository or distro acceptance. These credentials are not
   present today and **cannot be fabricated** — release stays blocked until they exist.
5. Publish the exact audited commit; verify remote SHA, checksums and release metadata
   before marking it ready.
6. Run `python3 scripts/check_public_tree.py` and a full-history Gitleaks scan; exclude
   local verification, auth/config, private paths and debug symbols from assets.

## Authoritative provider references

- [DeepSeek balance endpoint](https://api-docs.deepseek.com/api/get-user-balance/)
- [Codex CLI and ChatGPT plans](https://developers.openai.com/codex/)
- [Codex and ChatGPT plans (help article)](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan)
- [Hermes provider setup](https://hermes-agent.nousresearch.com/docs/integrations/providers)
