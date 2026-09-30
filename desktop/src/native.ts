import { invoke } from "@tauri-apps/api/core";
import type { Bridge, Mood, NativeStatus, Provider, ReadQuotaArgs, Snapshot, SmokeProof } from "./types";

/** Coerce an unknown field to string-or-null so no raw blob reaches the DOM. */
function str(v: unknown): string | null {
  return typeof v === "string" ? v : null;
}

function num(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v) ? v : null;
}

function bool(v: unknown): boolean | null {
  return typeof v === "boolean" ? v : null;
}

function mood(v: unknown): Mood {
  return v === "happy" || v === "steady" || v === "sleepy" || v === "asleep" || v === "unknown"
    ? v
    : "unknown";
}

function providerOf(v: unknown): Provider {
  return v === "openai-codex" ? "openai-codex" : "deepseek";
}

/** Sanitize a native snapshot into the frozen shape, masking malformed fields. */
export function asSnapshot(v: unknown): Snapshot {
  const o = (v ?? {}) as Record<string, unknown>;
  return {
    provider: providerOf(o.provider),
    source: str(o.source) ?? "",
    fetched_at: str(o.fetched_at) ?? "",
    mood: mood(o.mood),
    remaining_percent: num(o.remaining_percent),
    resets_at: str(o.resets_at),
    currency: str(o.currency),
    balance: str(o.balance),
    net_change: str(o.net_change),
    is_available: bool(o.is_available),
  };
}

export function asStatus(v: unknown): NativeStatus {
  const o = (v ?? {}) as Record<string, unknown>;
  return {
    demo: o.demo === true,
    version: str(o.version) ?? "",
    deepseek_configured: o.deepseek_configured === true,
    codex_available: o.codex_available === true,
    hermes_available: o.hermes_available === true,
    platform: str(o.platform) ?? "",
  };
}

/** The real native bridge, invoking Tauri commands from @tauri-apps/api/core. */
export function createNativeBridge(): Bridge {
  return {
    async getStatus(): Promise<NativeStatus> {
      return asStatus(await invoke("get_status"));
    },
    async readQuota(args: ReadQuotaArgs): Promise<Snapshot> {
      return asSnapshot(await invoke("read_quota", { provider: args.provider, codexSource: args.codexSource }));
    },
    async saveDeepSeekKey(key: string): Promise<void> {
      await invoke("save_deepseek_key", { key });
    },
    async deleteDeepSeekKey(): Promise<void> {
      await invoke("delete_deepseek_key");
    },
    async setDemo(enabled: boolean): Promise<NativeStatus> {
      return asStatus(await invoke("set_demo", { enabled }));
    },
    async quitApp(): Promise<void> {
      await invoke("quit_app");
    },
    async smokeContext(): Promise<boolean> {
      const result = await invoke("smoke_context");
      if (typeof result === "boolean") return result;
      const o = result as Record<string, unknown> | null;
      return o != null && o.enabled === true;
    },
    async reportSmoke(proof: SmokeProof): Promise<void> {
      await invoke("report_smoke", { proof });
    },
  };
}
