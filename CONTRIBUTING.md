# Contributing

Small, testable changes are easiest to review. Open an issue before adding a provider
or changing how authentication works.

## Run the checks

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

## Translations

UI strings and number/date formatting belong in the central localization code in
`Sources/QuotaCore/Localization.swift`. Add tests for every string and locale behavior.
Use the provider/language render matrix in `docs/development.md` to catch clipped labels.
Language selection must not change the account or trigger a network request.

## Provider adapters

A balance is not a quota. Missing data is not zero. Keep provider semantics explicit,
validate response fields, and fail closed on stale, invalid or unavailable data.
Never send credentials to configurable URLs, follow authorization-bearing redirects,
log authentication headers or use synthetic data as a live fallback.

Use tests with obvious dummy credentials, not realistic copied keys. New network behavior
needs a separate security review. Keep UI, parsing and authenticated requests separable.

By contributing, you agree your contributions are licensed under the repository's MIT license.
