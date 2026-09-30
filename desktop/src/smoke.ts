import type { Bridge, SmokeProof } from "./types";
import type { Store } from "./store";
import { compareDecimal, isValidDecimal } from "./decimal";

function raf(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

/** Wait until a read settles, then two frames so the UI has painted. */
async function waitForIdle(store: Store, timeoutMs = 5000): Promise<void> {
  const start = performance.now();
  while (store.busy && performance.now() - start < timeoutMs) {
    await raf();
  }
  await raf();
  await raf();
}

function textOf(testid: string): string | null {
  return document.querySelector(`[data-testid="${testid}"]`)?.textContent?.trim() ?? null;
}

/**
 * Native WebView smoke test (CLI `--demo --smoke-test`). Exercises the real
 * `read_quota` IPC for both providers, verifies the rendered DOM shows the
 * demo data, and reports proof only when every check passes. On any failure it
 * returns null and the caller reports nothing — the parent's timeout fails the
 * test. No fake JS fixtures are used in this path.
 */
export async function runSmokeTest(store: Store, bridge: Bridge): Promise<SmokeProof | null> {
  try {
    // 1. Real native reads for both providers (locale-independent raw values).
    const ds = await bridge.readQuota({ provider: "deepseek", codexSource: store.codexSource });
    const cx = await bridge.readQuota({ provider: "openai-codex", codexSource: store.codexSource });
    const dsBalanceOk =
      ds.balance != null && isValidDecimal(ds.balance) && compareDecimal(ds.balance, "42.50") === 0;
    if (!(ds.provider === "deepseek" && dsBalanceOk && ds.currency === "USD")) return null;
    if (!(cx.provider === "openai-codex" && cx.remaining_percent === 68)) return null;

    // 2. Force English so numeric assertions are locale-independent.
    store.selectLanguage("en");
    await raf();
    await raf();

    // 3. DeepSeek demo data in the DOM.
    store.selectProvider("deepseek");
    await waitForIdle(store);
    if (textOf("amount") !== "42.50" || textOf("currency") !== "USD") return null;

    // 4. Codex demo data in the DOM.
    store.selectProvider("openai-codex");
    await waitForIdle(store);
    if (textOf("percent") !== "68%") return null;

    // 5. The permanent demo banner is present.
    if (!document.querySelector('[data-testid="demo-banner"]')) return null;

    // 6. Language controls: verify the English then the Danish label.
    const enOk = textOf("language-label") === "Language";
    store.selectLanguage("da");
    await raf();
    await raf();
    const daOk = textOf("language-label") === "Sprog";
    if (!enOk || !daOk) return null;

    // 7. Leave the language choice back at the default system setting.
    store.selectLanguage("system");
    await raf();
    await raf();

    return {
      providers: ["deepseek", "openai-codex"],
      language: "en,da",
      demo: true,
      version: store.status?.version,
      platform: store.status?.platform,
    };
  } catch {
    return null;
  }
}
