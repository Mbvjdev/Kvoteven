import type { AppLanguage, Bridge, CodexSource, NativeStatus, Provider, Snapshot } from "./types";
import type { L10nKey } from "./i18n";
import { DEMO_ANCHOR } from "./demo";
import { sanitizeErrorCode } from "./errors";

export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** The only keys Kvoteven persists. Never account data, snapshots or keys. */
export const STORAGE_KEYS = {
  provider: "kvoteven.provider",
  language: "kvoteven.language",
  codexSource: "kvoteven.codex-source",
} as const;

const POLL_INTERVAL_MS = 30_000;
const REFRESH_PERIOD_MS = 300_000; // selected provider polls every 5 minutes
const MIN_ATTEMPT_GAP_MS = 15_000; // never more often than once per 15 seconds

type Listener = () => void;

/** Read without ever letting a throwing storage crash the quota UI. */
function safeGet(storage: StorageLike | null, key: string): string | null {
  try {
    return storage?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

/** Write without ever letting a throwing storage crash the quota UI. */
function safeSet(storage: StorageLike | null, key: string, value: string): void {
  try {
    storage?.setItem(key, value);
  } catch {
    /* storage unavailable — the setting stays in memory only */
  }
}

function readEnum<T extends string>(storage: StorageLike | null, key: string, allowed: readonly T[], fallback: T): T {
  const value = safeGet(storage, key);
  return value != null && (allowed as readonly string[]).includes(value) ? (value as T) : fallback;
}

export class Store {
  provider: Provider;
  language: AppLanguage;
  codexSource: CodexSource;

  demo = false;
  status: NativeStatus | null = null;
  snapshot: Snapshot | null = null;
  fetchFailed = false;
  /** Sanitized error code from the last quota read, so the UI can be actionable. */
  fetchErrorCode: string | null = null;
  busy = false;
  /** A key save/delete is in flight (guards duplicate submits). */
  settingsBusy = false;
  /** A demo-mode transition is in flight (guards duplicate toggles). */
  demoBusy = false;
  now: Date;

  /** Transient settings feedback, as a localized message key or an error code. */
  settingsMessage: L10nKey | null = null;
  settingsError: string | null = null;

  private readonly bridge: Bridge;
  private readonly storage: StorageLike | null;
  private readonly nowFn: () => Date;
  private listeners = new Set<Listener>();
  private lastAttemptAt: Partial<Record<Provider, number>> = {};
  private generation = 0;
  private timer: ReturnType<typeof setInterval> | null = null;
  /** The demo reference clock, pinned to the latest demo_synthetic fetched_at. */
  private demoNow: Date = new Date(DEMO_ANCHOR);

  constructor(opts: { bridge: Bridge; storage?: StorageLike | null; now?: () => Date }) {
    this.bridge = opts.bridge;
    this.storage = opts.storage ?? null;
    this.nowFn = opts.now ?? (() => new Date());
    this.provider = readEnum<Provider>(this.storage, STORAGE_KEYS.provider, ["deepseek", "openai-codex"], "deepseek");
    this.language = readEnum<AppLanguage>(this.storage, STORAGE_KEYS.language, ["system", "en", "da"], "system");
    this.codexSource = readEnum<CodexSource>(this.storage, STORAGE_KEYS.codexSource, ["auto", "codex", "hermes"], "auto");
    this.now = this.currentNow();
  }

  /**
   * The demo clock is frozen, but at the *snapshot's* fetched_at rather than
   * a hardcoded anchor. Native demo snapshots carry a wall-clock fetched_at,
   * so freezing at DEMO_ANCHOR made them look far-future/stale. Browser
   * fixtures still carry DEMO_ANCHOR, so their behaviour is unchanged.
   */
  private currentNow(): Date {
    return this.demo ? this.demoNow : this.nowFn();
  }

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private emit(): void {
    for (const fn of this.listeners) fn();
  }

  async init(): Promise<void> {
    this.status = await this.bridge.getStatus();
    this.demo = this.status.demo;
    this.now = this.currentNow();
    this.emit();
    await this.refresh();
    this.startTimer();
  }

  private startTimer(): void {
    if (this.timer) return;
    this.timer = setInterval(() => {
      this.now = this.currentNow();
      this.emit();
      if (this.demo) return; // demo never polls a network
      if (this.settingsBusy || this.demoBusy) return; // never race a config change
      const last = this.lastAttemptAt[this.provider];
      const nowMs = this.currentNow().getTime();
      if (last == null || nowMs - last >= REFRESH_PERIOD_MS) {
        void this.refresh();
      }
    }, POLL_INTERVAL_MS);
  }

  /** Persist a preference, but never in demo/smoke mode (must not touch user prefs). */
  private persist(key: string, value: string): void {
    if (this.demo) return;
    safeSet(this.storage, key, value);
  }

  selectProvider(p: Provider): void {
    if (p === this.provider) return;
    this.provider = p;
    this.persist(STORAGE_KEYS.provider, p);
    this.abandonReading();
  }

  selectLanguage(l: AppLanguage): void {
    if (l === this.language) return;
    this.language = l;
    this.persist(STORAGE_KEYS.language, l);
    this.emit();
  }

  selectCodexSource(s: CodexSource): void {
    if (s === this.codexSource) return;
    this.codexSource = s;
    this.persist(STORAGE_KEYS.codexSource, s);
    if (this.provider === "openai-codex") {
      this.abandonReading();
    } else {
      this.emit();
    }
  }

  /** Drop the current reading so no stale amount survives a config change. */
  private invalidateReading(): void {
    this.generation++;
    this.snapshot = null;
    this.fetchFailed = false;
    this.fetchErrorCode = null;
    this.busy = false;
    this.now = this.currentNow();
  }

  /** Clear the current reading at once and start a fresh read for the new selection. */
  private abandonReading(): void {
    this.invalidateReading();
    this.emit();
    void this.refresh(true);
  }

  async setDemo(enabled: boolean): Promise<void> {
    if (enabled === this.demo) return;
    if (this.demoBusy || this.settingsBusy) return;
    this.demoBusy = true;
    // Invalidate up front: an in-flight read from the old mode must never
    // land in the new mode.
    this.invalidateReading();
    this.emit();
    try {
      const status = await this.bridge.setDemo(enabled);
      this.status = status;
      this.demo = status.demo;
      this.settingsError = null;
    } catch (err) {
      // Fail closed: keep the prior mode, surface a sanitized error.
      this.settingsError = sanitizeErrorCode(err);
    }
    this.demoBusy = false;
    this.emit();
    await this.refresh(true);
  }

  async refresh(force = false): Promise<void> {
    if (this.busy || this.settingsBusy || this.demoBusy) return;
    const requested = this.provider;
    // Throttle on the real wall clock; demo reads are instant and synthetic.
    const nowMs = this.nowFn().getTime();
    const last = this.lastAttemptAt[requested];
    if (!this.demo && !force && last != null && nowMs - last < MIN_ATTEMPT_GAP_MS) return;
    this.lastAttemptAt[requested] = nowMs;
    const gen = this.generation;
    this.busy = true;
    this.emit();
    try {
      const snap = await this.bridge.readQuota({ provider: requested, codexSource: this.codexSource });
      if (gen !== this.generation) return; // stale: selection changed mid-flight
      // Never render a snapshot that leaked in from the other mode.
      const crossMode = this.demo ? snap.source !== "demo_synthetic" : snap.source === "demo_synthetic";
      if (crossMode) {
        this.snapshot = null;
        this.fetchFailed = true;
        this.fetchErrorCode = "unknown";
      } else {
        this.snapshot = snap;
        if (snap.source === "demo_synthetic") this.demoNow = new Date(snap.fetched_at);
        this.fetchFailed = false;
        this.fetchErrorCode = null;
      }
      this.busy = false;
    } catch (err) {
      if (gen !== this.generation) return;
      this.fetchFailed = true;
      this.fetchErrorCode = sanitizeErrorCode(err);
      this.busy = false;
    }
    this.now = this.currentNow();
    this.emit();
  }

  async saveKey(key: string): Promise<void> {
    const trimmed = key.trim();
    this.settingsMessage = null;
    this.settingsError = null;
    if (!trimmed) {
      this.emit();
      return;
    }
    if (this.settingsBusy) return; // no double-submit
    this.settingsBusy = true;
    // Invalidate up front so the old key's amount is never shown while saving.
    this.invalidateReading();
    this.emit();
    let ok = false;
    try {
      await this.bridge.saveDeepSeekKey(trimmed);
      this.settingsMessage = "deepseekKeySaved";
      if (this.status) this.status = { ...this.status, deepseek_configured: true };
      ok = true;
    } catch (err) {
      this.settingsError = sanitizeErrorCode(err);
    }
    this.settingsBusy = false;
    this.emit();
    if (ok) await this.refresh(true); // re-read the selected provider with the new key
  }

  async deleteKey(): Promise<void> {
    this.settingsMessage = null;
    this.settingsError = null;
    if (this.settingsBusy) return;
    this.settingsBusy = true;
    // Never keep the old amount on screen while the key is being removed.
    this.invalidateReading();
    this.emit();
    try {
      await this.bridge.deleteDeepSeekKey();
      this.settingsMessage = "deepseekKeyDeleted";
      if (this.status) this.status = { ...this.status, deepseek_configured: false };
    } catch (err) {
      this.settingsError = sanitizeErrorCode(err);
    }
    this.settingsBusy = false;
    this.emit();
    // Deliberately no refresh: DeepSeek stays unknown until a real new key.
  }

  quit(): void {
    void this.bridge.quitApp();
  }
}
