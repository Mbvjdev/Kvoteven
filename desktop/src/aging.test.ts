import { describe, it, expect } from "vitest";
import {
  FRESH_WINDOW_SECONDS,
  FUTURE_TOLERANCE_SECONDS,
  isFresh,
  codexView,
  deepseekView,
} from "./aging";
import type { Snapshot } from "./types";

function iso(date: Date): string {
  return date.toISOString();
}

function ago(seconds: number, from: Date): Date {
  return new Date(from.getTime() - seconds * 1000);
}

function snapshot(overrides: Partial<Snapshot> = {}): Snapshot {
  return {
    provider: "openai-codex",
    source: "app_server",
    fetched_at: iso(ago(60, new Date("2026-09-30T12:00:00Z"))),
    mood: "happy",
    remaining_percent: 68,
    resets_at: iso(new Date("2026-10-06T12:00:00Z")),
    currency: null,
    balance: null,
    net_change: null,
    is_available: null,
    ...overrides,
  };
}

const NOW = new Date("2026-09-30T12:00:00Z");

describe("isFresh", () => {
  it("accepts data within the 10 minute window", () => {
    expect(isFresh(iso(ago(1, NOW)), NOW)).toBe(true);
    expect(isFresh(iso(ago(FRESH_WINDOW_SECONDS - 1, NOW)), NOW)).toBe(true);
    expect(isFresh(iso(ago(FRESH_WINDOW_SECONDS, NOW)), NOW)).toBe(true);
  });

  it("rejects data older than 10 minutes", () => {
    expect(isFresh(iso(ago(FRESH_WINDOW_SECONDS + 1, NOW)), NOW)).toBe(false);
  });

  it("tolerates timestamps up to 60 seconds in the future", () => {
    expect(isFresh(iso(ago(-FUTURE_TOLERANCE_SECONDS, NOW)), NOW)).toBe(true);
    expect(isFresh(iso(ago(-FUTURE_TOLERANCE_SECONDS - 1, NOW)), NOW)).toBe(false);
  });
});

describe("codexView", () => {
  it("returns the weekly percentage and a happy mood when plenty remains", () => {
    expect(codexView(snapshot(), NOW, false)).toEqual({ remainingPercent: 68, mood: "happy", resetPassed: false });
  });

  it("maps the mood thresholds", () => {
    expect(codexView(snapshot({ remaining_percent: 40 }), NOW, false).mood).toBe("steady");
    expect(codexView(snapshot({ remaining_percent: 15 }), NOW, false).mood).toBe("sleepy");
    expect(codexView(snapshot({ remaining_percent: 0 }), NOW, false).mood).toBe("asleep");
    expect(codexView(snapshot({ remaining_percent: null }), NOW, false).mood).toBe("unknown");
  });

  it("masks a failed read", () => {
    const view = codexView(snapshot(), NOW, true);
    expect(view.remainingPercent).toBeNull();
    expect(view.mood).toBe("unknown");
  });

  it("masks a stale read", () => {
    const stale = snapshot({ fetched_at: iso(ago(FRESH_WINDOW_SECONDS + 5, NOW)) });
    expect(codexView(stale, NOW, false).remainingPercent).toBeNull();
    expect(codexView(stale, NOW, false).mood).toBe("unknown");
  });

  it("masks a reset that has already passed", () => {
    const reset = snapshot({ resets_at: iso(ago(1, NOW)) });
    const view = codexView(reset, NOW, false);
    expect(view.remainingPercent).toBeNull();
    expect(view.resetPassed).toBe(true);
    expect(view.mood).toBe("unknown");
  });

  it("masks when there is no snapshot at all", () => {
    expect(codexView(null, NOW, false).remainingPercent).toBeNull();
  });
});

describe("deepseekView", () => {
  function ds(overrides: Partial<Snapshot> = {}): Snapshot {
    return snapshot({
      provider: "deepseek",
      source: "deepseek_balance_api",
      currency: "USD",
      balance: "42.50",
      is_available: true,
      ...overrides,
    });
  }

  it("returns the balance and currency with a happy mood", () => {
    expect(deepseekView(ds(), NOW, false)).toEqual({ balance: "42.50", currency: "USD", mood: "happy" });
  });

  it("maps the balance thresholds", () => {
    expect(deepseekView(ds({ balance: "20.00" }), NOW, false).mood).toBe("steady");
    expect(deepseekView(ds({ balance: "5.00" }), NOW, false).mood).toBe("steady");
    expect(deepseekView(ds({ balance: "4.99" }), NOW, false).mood).toBe("sleepy");
    expect(deepseekView(ds({ balance: "0.00" }), NOW, false).mood).toBe("asleep");
  });

  it("shows asleep when the API is reported unavailable", () => {
    expect(deepseekView(ds({ is_available: false }), NOW, false).mood).toBe("asleep");
  });

  it("masks a missing balance", () => {
    const view = deepseekView(ds({ balance: null }), NOW, false);
    expect(view.balance).toBeNull();
    expect(view.mood).toBe("unknown");
  });

  it("masks failed, stale and missing reads", () => {
    expect(deepseekView(ds(), NOW, true).mood).toBe("unknown");
    const stale = ds({ fetched_at: iso(ago(FRESH_WINDOW_SECONDS + 5, NOW)) });
    expect(deepseekView(stale, NOW, false).mood).toBe("unknown");
    expect(deepseekView(null, NOW, false).balance).toBeNull();
  });
});
