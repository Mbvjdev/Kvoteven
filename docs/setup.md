# Setup and troubleshooting

## Requirements

- macOS 13 or later. The initial local release is exercised on Apple Silicon/macOS 15.
- Swift 5.9+ / Xcode Command Line Tools and Python 3 for the source build.
- For live accounts: a trusted Hermes Agent installation with the `usage` command and
  `--print-runtime-command`. Tested initially with Hermes v0.21.5+4533.g39faafb.
  This is a tested build identifier, not a promise that every earlier v0.21.5 build works.
- This preview uses the default Hermes profile and standard git install at
  `~/.hermes/hermes-agent`. Other profile layouts are not a supported setup yet.

Install Hermes using its [official documentation](https://hermes-agent.nousresearch.com/docs/).
Check these capabilities locally:

```sh
hermes usage --help
hermes --print-runtime-command
```

If a command is missing, update Hermes through its supported updater. Do not copy a
stranger's auth files or put your key into this project as a workaround.

## Codex

Use `hermes model` and select the ChatGPT / Codex subscription provider. Follow Hermes'
interactive login. Read the [current provider instructions](https://hermes-agent.nousresearch.com/docs/integrations/providers)
if authentication changes.

Check the connection locally:

```sh
hermes --profile default usage --provider openai-codex --json
```

This prints private account metrics. Do not upload the result. Missing weekly data
means this account cannot currently provide the weekly meter; Kvoteven will not guess.

## DeepSeek

Configure `DEEPSEEK_API_KEY` in Hermes' private `.env` using its setup, outside the repo.
The helper runs in Hermes' managed Python runtime, which supplies `httpx` and the
credential resolver. It performs only the documented balance GET request.

The balance is account-wide. It is not limited to calls made by Hermes or Kvoteven.

## App controls

Click the menu-bar pet to switch providers, change language, refresh or quit. The active
provider refreshes every five minutes; manual refreshes have a 15-second minimum gap.
Unknown/failed/stale readings are shown as a dash. Data older than ten minutes is stale.
Codex values also expire at their reported reset time.

System language means Danish when the selected system language is Danish, English otherwise.
You can force English or Danish in the panel without changing macOS itself.

The menu bar may be hidden in full-screen mode. Move the pointer to the top edge.

## When the app cannot find Hermes

The app checks `~/.local/bin/hermes`, the standard Hermes git installation, and `PATH`.
For a custom executable, launch from Terminal with `KVOTEVEN_HERMES` set to its absolute
path. This runs that executable with your permissions: use only a path you trust.
The DeepSeek adapter looks for the git source tree beside the resolved launcher, then falls
back to the standard Hermes installation. Wheel-only layouts are not supported.

## Distribution limitations

The app is locally ad-hoc signed, not Developer ID signed or notarized. Source builds
are the recommended route in this preview. We do not ask you to disable Gatekeeper.
There is no automatic updater or startup item.
