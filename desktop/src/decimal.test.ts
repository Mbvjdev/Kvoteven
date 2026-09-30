import { describe, it, expect } from "vitest";
import {
  isValidDecimal,
  compareDecimal,
  compareDecimalToNumber,
  formatDecimal,
} from "./decimal";

describe("isValidDecimal", () => {
  it("accepts integers and decimals", () => {
    expect(isValidDecimal("0")).toBe(true);
    expect(isValidDecimal("42.50")).toBe(true);
    expect(isValidDecimal("-42.50")).toBe(true);
    expect(isValidDecimal("1234567890")).toBe(true);
    expect(isValidDecimal("0.0001")).toBe(true);
  });

  it("rejects non-numeric and malformed values", () => {
    expect(isValidDecimal("")).toBe(false);
    expect(isValidDecimal("42.")).toBe(false);
    expect(isValidDecimal(".50")).toBe(false);
    expect(isValidDecimal("4a2")).toBe(false);
    expect(isValidDecimal("1,000")).toBe(false);
    expect(isValidDecimal("+42")).toBe(false);
    expect(isValidDecimal("42.5.0")).toBe(false);
  });

  it("rejects values longer than the provider limit", () => {
    expect(isValidDecimal("1".repeat(39))).toBe(false);
    expect(isValidDecimal("1".repeat(38))).toBe(true);
  });
});

describe("compareDecimal", () => {
  it("compares equal values ignoring trailing zeros", () => {
    expect(compareDecimal("42.50", "42.5")).toBe(0);
    expect(compareDecimal("0.10", "0.1")).toBe(0);
    expect(compareDecimal("7", "7.00")).toBe(0);
  });

  it("orders positive values", () => {
    expect(compareDecimal("10", "2")).toBe(1);
    expect(compareDecimal("2", "10")).toBe(-1);
    expect(compareDecimal("0.1", "0.2")).toBe(-1);
    expect(compareDecimal("0.15", "0.2")).toBe(-1);
    expect(compareDecimal("2.10", "2.2")).toBe(-1);
  });

  it("orders negative values", () => {
    expect(compareDecimal("-5", "3")).toBe(-1);
    expect(compareDecimal("-5", "-3")).toBe(-1);
    expect(compareDecimal("-3", "-5")).toBe(1);
    expect(compareDecimal("0", "-0.01")).toBe(1);
  });
});

describe("compareDecimalToNumber", () => {
  it("compares a decimal string against integer thresholds", () => {
    expect(compareDecimalToNumber("42.50", 20)).toBe(1);
    expect(compareDecimalToNumber("5", 5)).toBe(0);
    expect(compareDecimalToNumber("4.99", 5)).toBe(-1);
    expect(compareDecimalToNumber("0", 0)).toBe(0);
    expect(compareDecimalToNumber("0.01", 0)).toBe(1);
  });
});

describe("formatDecimal", () => {
  const us = { locale: "en-US", minFractionDigits: 2, maxFractionDigits: 2 };
  const dk = { locale: "da-DK", minFractionDigits: 2, maxFractionDigits: 2 };

  it("formats money with two decimals in en-US", () => {
    expect(formatDecimal("42.50", us)).toBe("42.50");
    expect(formatDecimal("42.5", us)).toBe("42.50");
    expect(formatDecimal("42", us)).toBe("42.00");
  });

  it("formats money with two decimals in da-DK", () => {
    expect(formatDecimal("42.50", dk)).toBe("42,50");
    expect(formatDecimal("42.5", dk)).toBe("42,50");
  });

  it("groups thousands using the locale separators", () => {
    expect(formatDecimal("1234.5", us)).toBe("1,234.50");
    expect(formatDecimal("1234.5", dk)).toBe("1.234,50");
    expect(formatDecimal("1000000", us)).toBe("1,000,000.00");
  });

  it("rounds half-up without floating point loss", () => {
    expect(formatDecimal("1.005", us)).toBe("1.01");
    expect(formatDecimal("1.004", us)).toBe("1.00");
    expect(formatDecimal("9.995", us)).toBe("10.00");
    expect(formatDecimal("0.999", us)).toBe("1.00");
  });

  it("preserves the sign", () => {
    expect(formatDecimal("-42.50", us)).toBe("-42.50");
    expect(formatDecimal("-0.50", us)).toBe("-0.50");
  });

  it("never corrupts large decimals through float coercion", () => {
    // 0.1 + 0.2 === 0.30000000000000004 in floats; string path must not drift
    expect(formatDecimal("90071992547409.93", us)).toBe("90,071,992,547,409.93");
  });

  it("honours a higher max fraction digit count", () => {
    const opts = { locale: "en-US", minFractionDigits: 2, maxFractionDigits: 6 };
    expect(formatDecimal("42.123456", opts)).toBe("42.123456");
    expect(formatDecimal("42.1", opts)).toBe("42.10");
    expect(formatDecimal("42.1234567", opts)).toBe("42.123457");
  });
});
