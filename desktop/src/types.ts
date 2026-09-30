/**
 * Shared types for the Kvoteven desktop frontend.
 *
 * These mirror the frozen native IPC contract (snake_case snapshot fields,
 * camelCase invoke arguments). The native backend owns canonical money
 * calculations; the frontend owns display, aging and locale formatting.
 */

export type Provider = "deepseek" | "openai-codex";
export type CodexSource = "auto" | "codex" | "hermes";
export type Mood = "happy" | "steady" | "sleepy" | "asleep" | "unknown";

/** User-facing language choice. `.system` resolves to en/da at render time. */
export type AppLanguage = "system" | "en" | "da";
/** Resolved UI language: only these two are ever rendered. */
export type UILanguage = "en" | "da";

export interface NativeStatus {
  demo: boolean;
  version: string;
  deepseek_configured: boolean;
  codex_available: boolean;
  hermes_available: boolean;
  platform: string;
}

/** Frozen native Snapshot. snake_case. nullable value fields mean "mask this". */
export interface Snapshot {
  provider: Provider;
  source: string;
  fetched_at: string; // ISO 8601
  mood: Mood;
  remaining_percent: number | null;
  resets_at: string | null;
  currency: string | null;
  balance: string | null; // decimal string
  net_change: string | null; // decimal string
  is_available: boolean | null;
}

export interface ReadQuotaArgs {
  provider: Provider;
  codexSource: CodexSource;
}

export interface SmokeProof {
  providers: Provider[];
  language: string;
  demo: boolean;
  version?: string;
  platform?: string;
}

/**
 * The boundary between the renderer and the world outside. Native mode wraps
 * Tauri `invoke`; browser preview mode uses an explicitly synthetic bridge so
 * demo data can never be mistaken for a live read.
 */
export interface Bridge {
  getStatus(): Promise<NativeStatus>;
  readQuota(args: ReadQuotaArgs): Promise<Snapshot>;
  saveDeepSeekKey(key: string): Promise<void>;
  deleteDeepSeekKey(): Promise<void>;
  setDemo(enabled: boolean): Promise<NativeStatus>;
  quitApp(): Promise<void>;
  smokeContext(): Promise<boolean>;
  reportSmoke(proof: SmokeProof): Promise<void>;
}
