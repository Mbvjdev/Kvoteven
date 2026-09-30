import { describe, it, expect } from "vitest";
import {
  resolveLanguage,
  resolveSystemLanguage,
  L10n,
  en as enTable,
  da as daTable,
} from "./i18n";

describe("resolveLanguage", () => {
  it("resolves system to danish for danish systems", () => {
    expect(resolveLanguage("system", "da-DK")).toBe("da");
    expect(resolveLanguage("system", "da")).toBe("da");
  });

  it("resolves system to english for everything else", () => {
    expect(resolveLanguage("system", "en-US")).toBe("en");
    expect(resolveLanguage("system", "de-DE")).toBe("en");
    expect(resolveLanguage("system", "")).toBe("en");
  });

  it("keeps an explicit language regardless of system", () => {
    expect(resolveLanguage("en", "da-DK")).toBe("en");
    expect(resolveLanguage("da", "en-US")).toBe("da");
  });
});

describe("resolveSystemLanguage", () => {
  it("picks danish when the first preference is danish", () => {
    expect(resolveSystemLanguage(["da-DK", "en-US"])).toBe("da");
  });

  it("falls back to english otherwise", () => {
    expect(resolveSystemLanguage(["en-US", "da-DK"])).toBe("en");
    expect(resolveSystemLanguage([])).toBe("en");
  });
});

describe("L10n string tables", () => {
  it("returns the correct strings in each language", () => {
    const en = new L10n("en");
    const da = new L10n("da");
    expect(en.text("apiBalance")).toBe("API balance");
    expect(da.text("apiBalance")).toBe("API-saldo");
    expect(en.text("demoBanner")).toBe("Demo · sample data");
    expect(da.text("demoBanner")).toBe("Demo · eksempeldata");
    expect(en.text("languageLabel")).toBe("Language");
    expect(da.text("languageLabel")).toBe("Sprog");
  });

  it("has a complete danish table covering every english key", () => {
    expect(Object.keys(daTable).length).toBe(Object.keys(enTable).length);
    for (const key of Object.keys(enTable)) {
      expect(daTable[key as keyof typeof enTable], `missing da key: ${key}`).toBeTruthy();
    }
  });

  it("localizes mood messages", () => {
    const en = new L10n("en");
    expect(en.moodMessage("happy")).toBe("Ready for more ideas.");
    expect(en.moodMessage("unknown")).toBe("Waiting for fresh data.");
    expect(en.deepSeekMoodMessage("asleep")).toBe("Time to top up a little.");
  });

  it("formats interpolated strings", () => {
    const en = new L10n("en");
    const da = new L10n("da");
    expect(en.resetInDays(2, 3)).toBe("Resets in 2 d 3 h");
    expect(da.resetInDays(2, 3)).toBe("Nulstilles om 2 d 3 t");
    expect(en.resetInHours(4, 5)).toBe("Resets in 4 h 5 min");
    expect(en.sleepyThreshold("USD")).toBe("Sleepy below 5 USD · no automatic top-up");
    expect(en.percentLeft(68)).toBe("68% left");
  });
});
