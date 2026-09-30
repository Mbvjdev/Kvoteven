import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { Store, STORAGE_KEYS, type StorageLike } from "./store";
import type { Bridge, NativeStatus, Provider, ReadQuotaArgs, Snapshot } from "./types";
import { DEMO_ANCHOR } from "./demo";

const T0 = "2026-09-30T12:00:00Z";

function snap(provider: Provider, overrides: Partial<Snapshot> = {}): Snapshot {
  return {
    provider,
    source: provider === "deepseek" ? "deepseek_balance_api" : "app_server",
    fetched_at: new Date(Date.parse(T0) - 1000).toISOString(),
    mood: "happy",
    remaining_percent: provider === "openai-codex" ? 68 : null,
    resets_at: provider === "openai-codex" ? "2026-10-06T12:00:00Z" : null,
    currency: provider === "deepseek" ? "USD" : null,
    balance: provider === "deepseek" ? "42.50" : null,
    net_change: null,
    is_available: true,
    ...overrides,
  };
}

const baseStatus: NativeStatus = {
  demo: false,
  version: "0.4.0",
  deepseek_configured: false,
  codex_available: true,
  hermes_available: true,
  platform: "test",
};

function makeBridge() {
  const readQuota = vi.fn(async (args: ReadQuotaArgs) => snap(args.provider));
  const getStatus = vi.fn(async (): Promise<NativeStatus> => ({ ...baseStatus }));
  const setDemo = vi.fn(async (enabled: boolean): Promise<NativeStatus> => ({ ...baseStatus, demo: enabled }));
  const saveDeepSeekKey = vi.fn(async () => {});
  const deleteDeepSeekKey = vi.fn(async () => {});
  const quitApp = vi.fn(async () => {});
  const smokeContext = vi.fn(async () => false);
  const reportSmoke = vi.fn(async () => {});
  const bridge: Bridge = { getStatus, readQuota, saveDeepSeekKey, deleteDeepSeekKey, setDemo, quitApp, smokeContext, reportSmoke };
  return { bridge, readQuota, getStatus, setDemo, saveDeepSeekKey, deleteDeepSeekKey, quitApp, smokeContext, reportSmoke };
}

function makeStorage(initial: Record<string, string> = {}) {
  const map = new Map(Object.entries(initial));
  const storage = {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => { map.set(k, v); },
    map,
  };
  return storage;
}

function makeStore(bridge: Bridge, storage = makeStorage()) {
  const store = new Store({ bridge, storage, now: () => new Date() });
  return store;
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(T0));
});
afterEach(() => {
  vi.useRealTimers();
});

describe("Store persistence", () => {
  it("loads persisted provider, language and codex source", () => {
    const storage = makeStorage({
      [STORAGE_KEYS.provider]: "openai-codex",
      [STORAGE_KEYS.language]: "da",
      [STORAGE_KEYS.codexSource]: "hermes",
    });
    const { bridge } = makeBridge();
    const store = makeStore(bridge, storage);
    expect(store.provider).toBe("openai-codex");
    expect(store.language).toBe("da");
    expect(store.codexSource).toBe("hermes");
  });

  it("falls back to safe defaults when nothing is persisted", () => {
    const { bridge } = makeBridge();
    const store = makeStore(bridge);
    expect(store.provider).toBe("deepseek");
    expect(store.language).toBe("system");
    expect(store.codexSource).toBe("auto");
  });

  it("persists only provider, language and codex source", async () => {
    const storage = makeStorage();
    const { bridge } = makeBridge();
    const store = makeStore(bridge, storage);
    await store.init();
    store.selectProvider("openai-codex");
    store.selectLanguage("en");
    store.selectCodexSource("codex");
    for (const key of storage.map.keys()) {
      expect([STORAGE_KEYS.provider, STORAGE_KEYS.language, STORAGE_KEYS.codexSource]).toContain(key);
    }
    // snapshot, balance and keys are never persisted
    expect([...storage.map.values()].join(" ")).not.toContain("42.50");
    expect([...storage.map.values()].join(" ")).not.toContain("deepseek_balance_api");
  });
});

describe("Store init and demo", () => {
  it("freezes the clock at the demo anchor in demo mode and never polls", async () => {
    const { bridge, readQuota, getStatus } = makeBridge();
    getStatus.mockResolvedValue({ ...baseStatus, demo: true });
    const store = makeStore(bridge);
    await store.init();
    expect(store.demo).toBe(true);
    expect(store.now.getTime()).toBe(Date.parse(DEMO_ANCHOR));
    expect(readQuota).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(3600_000); // one hour
    expect(readQuota).toHaveBeenCalledTimes(1); // demo never polls
    expect(store.now.getTime()).toBe(Date.parse(DEMO_ANCHOR));
  });

  it("loads the initial snapshot for the selected provider", async () => {
    const { bridge, readQuota } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    expect(readQuota).toHaveBeenCalledWith({ provider: "deepseek", codexSource: "auto" });
    expect(store.snapshot?.provider).toBe("deepseek");
    expect(store.fetchFailed).toBe(false);
  });
});

describe("Store provider switching and races", () => {
  it("clears the snapshot immediately and refetches on provider switch", async () => {
    const { bridge, readQuota } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    expect(store.snapshot).not.toBeNull();
    store.selectProvider("openai-codex");
    // cleared synchronously before the new read resolves
    expect(store.snapshot).toBeNull();
    await Promise.resolve();
    await Promise.resolve();
    expect(readQuota).toHaveBeenLastCalledWith({ provider: "openai-codex", codexSource: "auto" });
    expect(store.snapshot?.provider).toBe("openai-codex");
  });

  it("discards a stale response when the provider changes mid-flight", async () => {
    const { bridge, readQuota } = makeBridge();
    let resolveDeepseek!: (s: Snapshot) => void;
    readQuota.mockImplementation((args: ReadQuotaArgs) =>
      args.provider === "deepseek"
        ? new Promise<Snapshot>((res) => { resolveDeepseek = res; })
        : Promise.resolve(snap("openai-codex")),
    );
    const store = makeStore(bridge);
    const initPromise = store.init(); // deepseek read hangs
    await Promise.resolve();
    store.selectProvider("openai-codex"); // switch while deepseek pending
    await Promise.resolve();
    await Promise.resolve();
    // stale deepseek resolves after the switch
    resolveDeepseek(snap("deepseek"));
    await Promise.resolve();
    await Promise.resolve();
    expect(store.snapshot?.provider).toBe("openai-codex"); // stale response ignored
    await initPromise;
  });
});

describe("Store throttling and polling", () => {
  it("enforces the 15 second minimum between attempts", async () => {
    const { bridge, readQuota } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    expect(readQuota).toHaveBeenCalledTimes(1);
    await store.refresh(); // immediate retry within 15s
    expect(readQuota).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(16_000);
    await store.refresh();
    expect(readQuota).toHaveBeenCalledTimes(2);
  });

  it("polls the selected provider after five minutes", async () => {
    const { bridge, readQuota } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    expect(readQuota).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(300_000);
    expect(readQuota).toHaveBeenCalledTimes(2);
  });

  it("updates the clock every tick so stale data gets masked", async () => {
    const { bridge, readQuota } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    const before = store.now.getTime();
    vi.advanceTimersByTime(30_000);
    expect(store.now.getTime()).toBe(before + 30_000);
    expect(readQuota).toHaveBeenCalledTimes(1); // tick alone does not fetch
  });
});

describe("Store failures and settings", () => {
  it("masks the number when a read fails", async () => {
    const { bridge, readQuota } = makeBridge();
    readQuota.mockRejectedValue({ code: "network" });
    const store = makeStore(bridge);
    await store.init();
    expect(store.fetchFailed).toBe(true);
  });

  it("saves the DeepSeek key and marks it configured", async () => {
    const { bridge, saveDeepSeekKey } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    await store.saveKey("  sk-12345  ");
    expect(saveDeepSeekKey).toHaveBeenCalledWith("sk-12345");
    expect(store.status?.deepseek_configured).toBe(true);
    expect(store.settingsMessage).toBe("deepseekKeySaved");
  });

  it("surfaces a sanitized keyring error without claiming success", async () => {
    const { bridge, saveDeepSeekKey } = makeBridge();
    saveDeepSeekKey.mockRejectedValue({ code: "keyring_unavailable" });
    const store = makeStore(bridge);
    await store.init();
    await store.saveKey("sk-12345");
    expect(store.settingsError).toBe("keyring_unavailable");
    expect(store.settingsMessage).toBeNull();
    expect(store.status?.deepseek_configured).toBe(false);
  });

  it("deletes the stored key and marks it unconfigured", async () => {
    const { bridge, deleteDeepSeekKey, getStatus } = makeBridge();
    getStatus.mockResolvedValue({ ...baseStatus, deepseek_configured: true });
    const store = makeStore(bridge);
    await store.init();
    await store.deleteKey();
    expect(deleteDeepSeekKey).toHaveBeenCalled();
    expect(store.status?.deepseek_configured).toBe(false);
    expect(store.settingsMessage).toBe("deepseekKeyDeleted");
  });

  it("toggles demo through the backend and clears the current reading", async () => {
    const { bridge, setDemo, readQuota } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    expect(store.demo).toBe(false);
    await store.setDemo(true);
    expect(setDemo).toHaveBeenCalledWith(true);
    expect(store.demo).toBe(true);
    expect(readQuota.mock.calls.length).toBeGreaterThanOrEqual(2);
  });
});

function demoSnap(provider: Provider, fetchedAt: string): Snapshot {
  return {
    provider,
    source: "demo_synthetic",
    fetched_at: fetchedAt,
    mood: "happy",
    remaining_percent: provider === "openai-codex" ? 68 : null,
    resets_at: provider === "openai-codex" ? "2026-10-05T19:05:02Z" : null,
    currency: provider === "deepseek" ? "USD" : null,
    balance: provider === "deepseek" ? "42.50" : null,
    net_change: null,
    is_available: true,
  };
}

describe("Store native demo clock", () => {
  it("pins the demo clock to the snapshot's actual fetched_at, not a fixed anchor", async () => {
    const fetched = "2026-09-30T10:00:00Z";
    const { bridge, readQuota, getStatus } = makeBridge();
    getStatus.mockResolvedValue({ ...baseStatus, demo: true });
    readQuota.mockResolvedValue(demoSnap("deepseek", fetched));
    const store = makeStore(bridge);
    await store.init();
    expect(store.demo).toBe(true);
    expect(store.snapshot?.source).toBe("demo_synthetic");
    expect(store.now.getTime()).toBe(Date.parse(fetched));
  });

  it("never lets a live snapshot leak into demo mode", async () => {
    const { bridge, getStatus } = makeBridge(); // readQuota returns a live source
    getStatus.mockResolvedValue({ ...baseStatus, demo: true });
    const store = makeStore(bridge);
    await store.init();
    expect(store.snapshot).toBeNull();
    expect(store.fetchFailed).toBe(true);
  });

  it("never lets a demo_synthetic snapshot leak into live mode", async () => {
    const { bridge, readQuota } = makeBridge();
    readQuota.mockResolvedValue(demoSnap("deepseek", "2026-09-30T10:00:00Z"));
    const store = makeStore(bridge); // demo defaults to false
    await store.init();
    expect(store.snapshot).toBeNull();
    expect(store.fetchFailed).toBe(true);
  });
});

describe("Store key changes invalidate the reading", () => {
  it("clears the current reading synchronously when a key is saved", async () => {
    const { bridge, readQuota } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    expect(store.snapshot?.balance).toBe("42.50");
    const saveP = store.saveKey("sk-new-key");
    expect(store.snapshot).toBeNull(); // cleared at START, before the await resolves
    await saveP;
    expect(store.snapshot?.balance).toBe("42.50"); // refreshed with the new key
    expect(readQuota.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it("delete keeps DeepSeek unknown and does not refetch", async () => {
    const { bridge, readQuota, getStatus } = makeBridge();
    getStatus.mockResolvedValue({ ...baseStatus, deepseek_configured: true });
    const store = makeStore(bridge);
    await store.init();
    expect(store.snapshot?.balance).toBe("42.50");
    const calls = readQuota.mock.calls.length;
    const delP = store.deleteKey();
    expect(store.snapshot).toBeNull(); // cleared at START
    await delP;
    expect(store.snapshot).toBeNull(); // still unknown: never re-reads the old amount
    expect(readQuota.mock.calls.length).toBe(calls); // no refetch
    expect(store.status?.deepseek_configured).toBe(false);
    expect(store.settingsMessage).toBe("deepseekKeyDeleted");
  });

  it("discards a stale in-flight read after a key save invalidates", async () => {
    const { bridge, readQuota } = makeBridge();
    let resolveDeepseek!: (s: Snapshot) => void;
    let hang = true;
    readQuota.mockImplementation((args: ReadQuotaArgs) => {
      if (args.provider === "deepseek" && hang) {
        hang = false;
        return new Promise<Snapshot>((res) => { resolveDeepseek = res; });
      }
      return Promise.resolve(snap(args.provider));
    });
    const store = makeStore(bridge);
    const initP = store.init(); // first deepseek read hangs
    await Promise.resolve();
    const saveP = store.saveKey("sk-new"); // invalidates generation while read is in flight
    await Promise.resolve();
    resolveDeepseek(snap("deepseek")); // stale read resolves after invalidation
    await saveP;
    await initP;
    expect(store.settingsMessage).toBe("deepseekKeySaved");
    expect(store.snapshot?.provider).toBe("deepseek"); // from the post-save refresh
  });

  it("ignores a duplicate save while one is already in flight", async () => {
    const { bridge, saveDeepSeekKey } = makeBridge();
    let resolveSave!: () => void;
    saveDeepSeekKey.mockImplementation(() => new Promise<void>((res) => { resolveSave = res; }));
    const store = makeStore(bridge);
    await store.init();
    const p1 = store.saveKey("sk-first");
    const p2 = store.saveKey("sk-second"); // ignored: settingsBusy
    resolveSave();
    await p1;
    await p2;
    expect(saveDeepSeekKey).toHaveBeenCalledTimes(1);
    expect(saveDeepSeekKey).toHaveBeenCalledWith("sk-first");
  });
});

describe("Store demo mode transitions", () => {
  it("keeps the prior mode when setDemo fails (fail closed)", async () => {
    const { bridge, setDemo } = makeBridge();
    const store = makeStore(bridge);
    await store.init();
    expect(store.demo).toBe(false);
    setDemo.mockRejectedValue({ code: "internal" });
    await store.setDemo(true);
    expect(store.demo).toBe(false);
    expect(store.settingsError).toBe("internal");
  });

  it("discards an in-flight live read when demo mode flips", async () => {
    const { bridge, readQuota, setDemo } = makeBridge();
    let resolveLive!: (s: Snapshot) => void;
    let first = true;
    readQuota.mockImplementation((args: ReadQuotaArgs) => {
      if (first) {
        first = false;
        return new Promise<Snapshot>((res) => { resolveLive = res; });
      }
      return Promise.resolve(demoSnap(args.provider, "2026-09-30T10:00:00Z"));
    });
    setDemo.mockResolvedValue({ ...baseStatus, demo: true });
    const store = makeStore(bridge);
    const initP = store.init(); // first live read hangs
    await Promise.resolve();
    const demoP = store.setDemo(true); // invalidates generation at START
    await Promise.resolve();
    await Promise.resolve();
    resolveLive(snap("deepseek")); // stale live read resolves after invalidation
    await Promise.resolve();
    await Promise.resolve();
    await demoP;
    await initP;
    expect(store.demo).toBe(true);
    expect(store.snapshot?.source).toBe("demo_synthetic"); // only demo data landed
  });
});

describe("Store demo preference isolation", () => {
  it("never persists provider/language/source while in demo", async () => {
    const storage = makeStorage();
    const { bridge, getStatus } = makeBridge();
    getStatus.mockResolvedValue({ ...baseStatus, demo: true });
    const store = makeStore(bridge, storage);
    await store.init();
    store.selectProvider("openai-codex");
    store.selectLanguage("da");
    store.selectCodexSource("hermes");
    expect(storage.map.size).toBe(0);
  });

  it("persists preferences in live mode", async () => {
    const storage = makeStorage();
    const { bridge } = makeBridge();
    const store = makeStore(bridge, storage);
    await store.init();
    store.selectProvider("openai-codex");
    store.selectLanguage("da");
    store.selectCodexSource("hermes");
    expect(storage.map.get(STORAGE_KEYS.provider)).toBe("openai-codex");
    expect(storage.map.get(STORAGE_KEYS.language)).toBe("da");
    expect(storage.map.get(STORAGE_KEYS.codexSource)).toBe("hermes");
  });
});

describe("Store storage failure fallback", () => {
  it("falls back to in-memory settings when storage throws", () => {
    const throwing: StorageLike = {
      getItem: () => { throw new Error("denied"); },
      setItem: () => { throw new Error("quota"); },
    };
    const { bridge } = makeBridge();
    const store = new Store({ bridge, storage: throwing, now: () => new Date(T0) });
    expect(store.provider).toBe("deepseek");
    expect(store.language).toBe("system");
    expect(store.codexSource).toBe("auto");
    expect(() => store.selectProvider("openai-codex")).not.toThrow();
    expect(store.provider).toBe("openai-codex");
  });

  it("does not crash init when storage reads throw", async () => {
    const throwing: StorageLike = {
      getItem: () => { throw new Error("denied"); },
      setItem: () => {},
    };
    const { bridge } = makeBridge();
    const store = new Store({ bridge, storage: throwing, now: () => new Date(T0) });
    await store.init();
    expect(store.snapshot?.provider).toBe("deepseek");
  });
});

describe("Store quota error codes", () => {
  it("records the sanitized error code on a failed read", async () => {
    const { bridge, readQuota } = makeBridge();
    readQuota.mockRejectedValue({ code: "not_configured" });
    const store = makeStore(bridge);
    await store.init();
    expect(store.fetchFailed).toBe(true);
    expect(store.fetchErrorCode).toBe("not_configured");
  });

  it("sanitizes an unknown read error to the generic fallback", async () => {
    const { bridge, readQuota } = makeBridge();
    readQuota.mockRejectedValue({ code: "network" });
    const store = makeStore(bridge);
    await store.init();
    expect(store.fetchErrorCode).toBe("unknown");
  });
});
