import { createNativeBridge } from "./native";
import { demoSnapshot, demoStatus } from "./demo";
import type { Bridge, NativeStatus, Provider, ReadQuotaArgs, Snapshot, SmokeProof } from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    /** Test-only synthetic bridge injected by Playwright via addInitScript. */
    __kvotevenBridge?: Bridge;
  }
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** A clearly synthetic, network-free bridge for the browser preview (?demo=1). */
export function createSyntheticDemoBridge(): Bridge {
  return {
    async getStatus(): Promise<NativeStatus> {
      return demoStatus();
    },
    async readQuota(args: ReadQuotaArgs): Promise<Snapshot> {
      return demoSnapshot(args.provider);
    },
    async saveDeepSeekKey(): Promise<void> {
      // A browser preview has no OS keyring and must never claim a saved key.
      throw { code: "keyring_unavailable" };
    },
    async deleteDeepSeekKey(): Promise<void> {},
    async setDemo(): Promise<NativeStatus> {
      return demoStatus();
    },
    async quitApp(): Promise<void> {},
    async smokeContext(): Promise<boolean> {
      return false;
    },
    async reportSmoke(_proof: SmokeProof): Promise<void> {},
  };
}

export type AppMode = "native" | "browser-preview" | "blocked";

export interface Detected {
  bridge: Bridge;
  mode: AppMode;
}

/**
 * Resolve the bridge. A test-injected synthetic bridge wins; otherwise the
 * native Tauri bridge; otherwise the browser preview requires `?demo=1` and is
 * otherwise blocked (never a silent live fallback).
 */
export function detectBridge(): Detected {
  if (typeof window !== "undefined" && window.__kvotevenBridge) {
    return { bridge: window.__kvotevenBridge, mode: "browser-preview" };
  }
  if (isTauriRuntime()) {
    return { bridge: createNativeBridge(), mode: "native" };
  }
  const params = new URLSearchParams(window.location.search);
  if (params.get("demo") === "1") {
    return { bridge: createSyntheticDemoBridge(), mode: "browser-preview" };
  }
  return { bridge: createSyntheticDemoBridge(), mode: "blocked" };
}
