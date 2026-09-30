import { describe, it, expect } from "vitest";
import {
  formatMoneyAmount,
  formatNetChange,
  formatPercent,
  formatTime,
  formatResetDate,
  countdown,
} from "./format";

describe("formatMoneyAmount", () => {
  it("formats decimal strings with the locale and a dash for missing values", () => {
    expect(formatMoneyAmount("42.50", "en")).toBe("42.50");
    expect(formatMoneyAmount("42.50", "da")).toBe("42,50");
    expect(formatMoneyAmount(null, "en")).toBe("—");
    expect(formatMoneyAmount("not-a-number", "en")).toBe("—");
  });
});

describe("formatNetChange", () => {
  it("signs positive and negative changes without float loss", () => {
    expect(formatNetChange("1.50", "en")).toBe("+1.50");
    expect(formatNetChange("-1.50", "en")).toBe("-1.50");
    expect(formatNetChange("0.00", "en")).toBe("0.00");
    expect(formatNetChange("0.1", "en")).toBe("+0.10");
  });

  it("returns a dash for missing changes", () => {
    expect(formatNetChange(null, "en")).toBe("—");
  });
});

describe("formatPercent", () => {
  it("formats whole percentages", () => {
    expect(formatPercent(68)).toBe("68%");
    expect(formatPercent(0)).toBe("0%");
    expect(formatPercent(100)).toBe("100%");
  });

  it("shows sub-1% values as <1%", () => {
    expect(formatPercent(0.5)).toBe("<1%");
    expect(formatPercent(0.999)).toBe("<1%");
  });

  it("masks missing values", () => {
    expect(formatPercent(null)).toBe("—");
  });
});

describe("formatTime", () => {
  it("renders a short time in the selected locale", () => {
    expect(formatTime("2026-09-30T14:05:00Z", "en", "UTC")).toBe("2:05 PM");
    expect(formatTime("2026-09-30T14:05:00Z", "da", "UTC")).toBe("14.05");
  });

  it("returns a dash for missing timestamps", () => {
    expect(formatTime(null, "en")).toBe("—");
  });
});

describe("formatResetDate", () => {
  it("renders weekday, date and time", () => {
    expect(formatResetDate("2026-09-30T14:05:00Z", "en", "UTC")).toBe("Wed, Sep 30, 2:05 PM");
    expect(formatResetDate("2026-09-30T14:05:00Z", "da", "UTC")).toBe("ons. 30. sep., 14.05");
  });

  it("returns a dash for missing timestamps", () => {
    expect(formatResetDate(null, "en")).toBe("—");
  });
});

describe("countdown", () => {
  const NOW = new Date("2026-09-30T00:00:00Z");

  it("splits the remaining time into days, hours and minutes", () => {
    expect(countdown("2026-10-02T03:30:00Z", NOW)).toEqual({
      days: 2, hours: 3, minutes: 30, passed: false,
    });
  });

  it("reports a reset that has already passed", () => {
    expect(countdown("2026-09-29T00:00:00Z", NOW).passed).toBe(true);
  });

  it("uses hours and minutes only when less than a day remains", () => {
    expect(countdown("2026-09-30T05:07:00Z", NOW)).toEqual({
      days: 0, hours: 5, minutes: 7, passed: false,
    });
  });
});
