# Development and verification

## Offline checks

```sh
swift test
python3 -m unittest discover -s scripts -p 'test_*.py' -v
python3 scripts/build_app.py
```

## Public-safe screenshots

Only this explicit mode is suitable for public docs. All amounts are fictional,
the demo banner remains visible, and no credentials are read:

```sh
mkdir -p verification
.build/release/Kvoteven --render verification/codex-en.png --demo --provider openai-codex --language en
.build/release/Kvoteven --render verification/deepseek-en.png --demo --provider deepseek --language en
.build/release/Kvoteven --render verification/deepseek-da.png --demo --provider deepseek --language da
.build/release/Kvoteven --render verification/deepseek-dark.png --demo --provider deepseek --language en --dark
```

`--demo` also works for `--ui-smoke`. `--check --demo` is rejected so a fixture can never
be mistaken for a successful live account check. Render files in `verification/` are ignored;
copy only reviewed, explicitly marked demo images to public docs.

## Live checks (private local output)

These commands require configured accounts, use the real read-only provider paths and
print private account metrics. They are intentionally **not** run in public CI:

```sh
python3 scripts/check_app.py
python3 scripts/check_deepseek_app.py
build/Kvoteven.app/Contents/MacOS/Kvoteven --ui-smoke --provider openai-codex
build/Kvoteven.app/Contents/MacOS/Kvoteven --ui-smoke --provider deepseek
```

## Release checklist

1. Pass offline tests, packaged app checks, demo renders and a separate live check on a configured Mac.
2. Inspect both providers in English/Danish and light/dark appearance.
3. Review `git diff --cached` and run `python3 scripts/check_public_tree.py`.
4. Scan only the publication candidate tree, then the complete committed history, with Gitleaks.
5. Exclude local verification, auth/config, private paths and debug symbols from release assets.
6. Publish the exact audited commit; verify the remote SHA and CI result before marking it ready.

## Authoritative provider references

- [DeepSeek balance endpoint](https://api-docs.deepseek.com/api/get-user-balance/)
- [Codex and ChatGPT plans](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan)
- [Hermes provider setup](https://hermes-agent.nousresearch.com/docs/integrations/providers)
