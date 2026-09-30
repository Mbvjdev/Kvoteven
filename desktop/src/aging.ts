import { compareDecimalToNumber } from "./decimal";
import type { Mood, Snapshot } from "./types";

/** A reading is fresh for at most 10 minutes, and no more than 60s in the future. */
export const FRESH_WINDOW_SECONDS = 600;
export const FUTURE_TOLERANCE_SECONDS = 60;

/** now - fetchedAt in seconds. Negative means the reading is from the future. */
export function snapshotAgeSeconds(fetchedAt: string, now: Date): number {
  const fetched = new Date(fetchedAt).getTime();
  if (Number.isNaN(fetched)) return Number.POSITIVE_INFINITY;
  return (now.getTime() - fetched) / 1000;
}

/** True when the reading is neither stale nor implausibly far in the future. */
export function isFresh(fetchedAt: string, now: Date): boolean {
  const age = snapshotAgeSeconds(fetchedAt, now);
  return age >= -FUTURE_TOLERANCE_SECONDS && age <= FRESH_WINDOW_SECONDS;
}

export interface CodexView {
  remainingPercent: number | null;
  mood: Mood;
  resetPassed: boolean;
}

export interface DeepseekView {
  balance: string | null;
  currency: string | null;
  mood: Mood;
}

function codexMood(remaining: number): Mood {
  if (remaining > 50) return "happy";
  if (remaining > 20) return "steady";
  if (remaining > 0) return "sleepy";
  return "asleep";
}

function deepseekMood(balance: string, isAvailable: boolean | null): Mood {
  if (isAvailable === false) return "asleep";
  if (compareDecimalToNumber(balance, 20) > 0) return "happy";
  if (compareDecimalToNumber(balance, 5) >= 0) return "steady";
  if (compareDecimalToNumber(balance, 0) > 0) return "sleepy";
  return "asleep";
}

/**
 * Codex weekly quota view. Any failed, stale or already-passed reset masks the
 * number (null) and reads as `unknown` — never a reassuring old value.
 */
export function codexView(snapshot: Snapshot | null, now: Date, fetchFailed: boolean): CodexView {
  if (!snapshot || fetchFailed || !isFresh(snapshot.fetched_at, now)) {
    return { remainingPercent: null, mood: "unknown", resetPassed: false };
  }
  const resetPassed = snapshot.resets_at != null && new Date(snapshot.resets_at).getTime() <= now.getTime();
  if (resetPassed || snapshot.remaining_percent == null) {
    return { remainingPercent: null, mood: "unknown", resetPassed };
  }
  const remaining = snapshot.remaining_percent;
  return { remainingPercent: remaining, mood: codexMood(remaining), resetPassed: false };
}

/**
 * DeepSeek balance view. There is no weekly DeepSeek quota — only the account
 * balance, availability and net change. Failed/stale reads mask the balance.
 */
export function deepseekView(snapshot: Snapshot | null, now: Date, fetchFailed: boolean): DeepseekView {
  if (!snapshot || fetchFailed || !isFresh(snapshot.fetched_at, now)) {
    return { balance: null, currency: null, mood: "unknown" };
  }
  if (snapshot.balance == null) {
    return { balance: null, currency: snapshot.currency, mood: "unknown" };
  }
  return {
    balance: snapshot.balance,
    currency: snapshot.currency,
    mood: deepseekMood(snapshot.balance, snapshot.is_available),
  };
}
