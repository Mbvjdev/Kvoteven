# Contributing

Small, testable changes are easiest to review. Open an issue before adding a provider
or changing how authentication works.

The primary distribution is the Tauri desktop app in `desktop/`. The Swift app under
`Sources/` is a legacy build with its own test suite — don't mix the two build systems.

## Run the checks

Desktop app (primary):

```sh
cd desktop
npm ci --ignore-scripts
npm test          # Vitest unit tests
npm run build     # type-check + Vite production build
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run test:e2e  # Playwright browser tests (Linux CI)
```

Legacy Swift app (still preserved):

```sh
swift test
python3 -m unittest discover -s scripts -p 'test_*.py' -v
python3 scripts/build_app.py
```

The unit tests do not require accounts. Use `--demo` for screenshots and UI work.
Never share live verification output in an issue or pull request.

Before a commit, stage only the intended files and run:

```sh
python3 scripts/check_public_tree.py
gitleaks git --redact --no-banner .
```

The public-tree guard checks staged content. Gitleaks' Git scan checks committed history;
scan the candidate files before the first public push as well. Neither replaces review.
The source allow-list excludes `build/`, `node_modules/` and `target/`; only explicitly
approved images belong in public docs.

## Translations

Desktop UI strings live in `desktop/src/i18n.ts` (English and Danish side by side). Add tests
for every string and locale behavior in `desktop/src/i18n.test.ts` and `format.test.ts`.
Language selection must not change the account or trigger a network request.

The legacy Swift localization lives in `Sources/QuotaCore/Localization.swift` and only changes
when the legacy build changes.

## Provider adapters

A balance is not a quota. Missing data is not zero. Keep provider semantics explicit,
validate response fields, and fail closed on stale, invalid or unavailable data.

Desktop adapters are in `desktop/src-tauri/src/providers.rs`. Never send credentials to
configurable URLs, follow authorization-bearing redirects, log authentication headers or
use synthetic data as a live fallback. The DeepSeek key is only ever read from the OS keyring
(`desktop/src-tauri/src/vault.rs`); do not add a plaintext path.

Use tests with obvious dummy credentials, not realistic copied keys. New network behavior
needs a separate security review. Keep UI, parsing and authenticated requests separable.

By contributing, you agree your contributions are licensed under the repository's MIT license.
