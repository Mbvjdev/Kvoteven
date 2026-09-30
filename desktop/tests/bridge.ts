import type { Page } from "@playwright/test";
import type { Provider, Snapshot } from "../src/types";

export const ANCHOR = "2026-09-29T19:05:02Z";

export interface BridgeOptions {
  demo?: boolean;
  deepseekConfigured?: boolean;
  codexAvailable?: boolean;
  hermesAvailable?: boolean;
  deepseek?: Partial<Snapshot>;
  codex?: Partial<Snapshot>;
  /** Make readQuota reject for a provider with a sanitized code. */
  fail?: { provider: Provider; code?: string };
  keyringFail?: boolean;
}

function buildSnapshots(opts: BridgeOptions): Record<Provider, Snapshot> {
  const demo = opts.demo ?? true;
  const freshNow = new Date(Date.now() - 1000).toISOString();
  const freshReset = new Date(Date.now() + 6 * 86_400_000).toISOString();

  return {
    deepseek: {
      provider: "deepseek",
      source: demo ? "demo_synthetic" : "deepseek_balance_api",
      fetched_at: demo ? ANCHOR : freshNow,
      mood: "happy",
      remaining_percent: null,
      resets_at: null,
      currency: "USD",
      balance: "42.50",
      net_change: null,
      is_available: true,
      ...opts.deepseek,
    },
    "openai-codex": {
      provider: "openai-codex",
      source: demo ? "demo_synthetic" : "app_server",
      fetched_at: demo ? ANCHOR : freshNow,
      mood: "happy",
      remaining_percent: 68,
      resets_at: demo ? "2026-10-05T19:05:02Z" : freshReset,
      currency: null,
      balance: null,
      net_change: null,
      is_available: true,
      ...opts.codex,
    },
  };
}

/** Inject a clearly synthetic test bridge before the app boots. */
export async function injectBridge(page: Page, opts: BridgeOptions = {}): Promise<void> {
  const snapshots = buildSnapshots(opts);
  const demo = opts.demo ?? true;
  await page.addInitScript(
    ({ demo, deepseekConfigured, codexAvailable, hermesAvailable, snapshots, fail, keyringFail }) => {
      const status = {
        demo,
        version: "0.4.0",
        deepseek_configured: deepseekConfigured,
        codex_available: codexAvailable,
        hermes_available: hermesAvailable,
        platform: "test",
      };
      (window as unknown as { __kvotevenBridge: unknown }).__kvotevenBridge = {
        async getStatus() {
          return status;
        },
        async readQuota(args: { provider: Provider }) {
          if (fail && args.provider === fail.provider) {
            throw { code: fail.code || "unknown" };
          }
          return snapshots[args.provider];
        },
        async saveDeepSeekKey() {
          if (keyringFail) throw { code: "keyring_unavailable" };
        },
        async deleteDeepSeekKey() {},
        async setDemo(enabled: boolean) {
          return { ...status, demo: enabled };
        },
        async quitApp() {},
        async smokeContext() {
          return false;
        },
        async reportSmoke() {},
      };
    },
    {
      demo,
      deepseekConfigured: opts.deepseekConfigured ?? false,
      codexAvailable: opts.codexAvailable ?? true,
      hermesAvailable: opts.hermesAvailable ?? true,
      snapshots,
      fail: opts.fail ?? null,
      keyringFail: opts.keyringFail ?? false,
    },
  );
}
