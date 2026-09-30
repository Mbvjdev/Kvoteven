import type { Provider, Snapshot } from "./types";

/**
 * Fixed, clearly synthetic demo fixtures. They never touch the network or any
 * credential store, and `source: "demo_synthetic"` makes them impossible to
 * mistake for a live provider response. The anchor timestamp matches the
 * original Swift app so demo renders are deterministic.
 */
export const DEMO_ANCHOR = "2026-09-29T19:05:02Z";
export const DEMO_RESETS_AT = "2026-10-05T19:05:02Z"; // anchor + 6 days

/** Fixed synthetic DeepSeek balance: 42.50 USD. */
export function demoDeepSeek(): Snapshot {
  return {
    provider: "deepseek",
    source: "demo_synthetic",
    fetched_at: DEMO_ANCHOR,
    mood: "happy",
    remaining_percent: null,
    resets_at: null,
    currency: "USD",
    balance: "42.50",
    net_change: "0.00",
    is_available: true,
  };
}

/** Fixed synthetic Codex quota: 68% of the weekly window remaining. */
export function demoCodex(): Snapshot {
  return {
    provider: "openai-codex",
    source: "demo_synthetic",
    fetched_at: DEMO_ANCHOR,
    mood: "happy",
    remaining_percent: 68,
    resets_at: DEMO_RESETS_AT,
    currency: null,
    balance: null,
    net_change: null,
    is_available: true,
  };
}

export function demoSnapshot(provider: Provider): Snapshot {
  return provider === "deepseek" ? demoDeepSeek() : demoCodex();
}

export function demoStatus(): { demo: true; version: string; deepseek_configured: boolean; codex_available: boolean; hermes_available: boolean; platform: string } {
  return {
    demo: true,
    version: "0.4.0",
    deepseek_configured: false,
    codex_available: true,
    hermes_available: true,
    platform: "browser",
  };
}
