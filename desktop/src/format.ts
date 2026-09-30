import { formatDecimal, isValidDecimal } from "./decimal";
import type { UILanguage } from "./types";

const LOCALES: Record<UILanguage, string> = { en: "en-US", da: "da-DK" };

/** Format a money amount (2 decimals) or "—" when absent/invalid. */
export function formatMoneyAmount(decimalStr: string | null, language: UILanguage): string {
  return money(decimalStr, language, 2);
}

/** Format a net change with an explicit sign; max 6 fraction digits. */
export function formatNetChange(decimalStr: string | null, language: UILanguage): string {
  if (decimalStr == null) return "—";
  const value = money(decimalStr, language, 6);
  if (value === "—") return value;
  const negative = decimalStr.startsWith("-");
  const zero = /^[-]?0*\.?0*$/.test(decimalStr);
  if (zero) return value;
  return negative ? value : `+${value}`;
}

function money(decimalStr: string | null, language: UILanguage, maxFractionDigits: number): string {
  if (decimalStr == null || !isValidDecimal(decimalStr)) return "—";
  return formatDecimal(decimalStr, {
    locale: LOCALES[language],
    minFractionDigits: 2,
    maxFractionDigits,
  });
}

/** Format a percentage: sub-1% values render as "<1%", missing as "—". */
export function formatPercent(value: number | null): string {
  if (value == null) return "—";
  if (value > 0 && value < 1) return "<1%";
  return `${Math.round(value)}%`;
}

/** Short clock time in the selected locale. */
export function formatTime(iso: string | null, language: UILanguage, timeZone?: string): string {
  if (iso == null) return "—";
  const opts: Intl.DateTimeFormatOptions = { timeStyle: "short" };
  if (timeZone) opts.timeZone = timeZone;
  return new Intl.DateTimeFormat(LOCALES[language], opts).format(new Date(iso));
}

/** Full reset timestamp (weekday, date and time) in the selected locale. */
export function formatResetDate(iso: string | null, language: UILanguage, timeZone?: string): string {
  if (iso == null) return "—";
  const opts: Intl.DateTimeFormatOptions = {
    weekday: "short",
    day: "numeric",
    month: "short",
    hour: "numeric",
    minute: "numeric",
  };
  if (timeZone) opts.timeZone = timeZone;
  return new Intl.DateTimeFormat(LOCALES[language], opts).format(new Date(iso));
}

export interface Countdown {
  days: number;
  hours: number;
  minutes: number;
  passed: boolean;
}

/** Time remaining until a reset, floored into days/hours/minutes. */
export function countdown(resetsAt: string, now: Date): Countdown {
  const seconds = Math.floor((new Date(resetsAt).getTime() - now.getTime()) / 1000);
  if (seconds <= 0) return { days: 0, hours: 0, minutes: 0, passed: true };
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return { days, hours, minutes, passed: false };
}
