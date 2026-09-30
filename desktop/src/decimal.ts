/**
 * Decimal-string arithmetic and formatting for money values.
 *
 * The native backend owns canonical money calculations and hands the frontend
 * decimal *strings* (e.g. "42.50"). Nothing here may round-trip through a
 * JavaScript `number`, because that reintroduces binary floating-point loss
 * (0.1 + 0.2 !== 0.3). All parsing, comparison and rounding is done on digit
 * strings; grouping/locale output uses `Intl` only for the integer part
 * (formatted as a `BigInt`, which `Intl.NumberFormat` supports natively).
 */

const DECIMAL_PATTERN = /^-?[0-9]+(?:\.[0-9]+)?$/;

/** True when `s` is a provider-valid decimal string (sign, digits, optional fraction). */
export function isValidDecimal(s: string): boolean {
  return s.length <= 38 && DECIMAL_PATTERN.test(s);
}

interface Parts {
  negative: boolean;
  int: string; // integer digits, no leading zeros, "0" when zero
  frac: string; // fraction digits, may be ""
}

function split(s: string): Parts {
  const negative = s.startsWith("-");
  const body = negative ? s.slice(1) : s;
  const dot = body.indexOf(".");
  const intRaw = dot === -1 ? body : body.slice(0, dot);
  const frac = dot === -1 ? "" : body.slice(dot + 1);
  const int = intRaw.replace(/^0+(?=.)/, ""); // strip leading zeros, keep a single "0"
  return { negative, int, frac };
}

function padEndToLength(a: string, b: string): [string, string] {
  const len = Math.max(a.length, b.length);
  return [a.padEnd(len, "0"), b.padEnd(len, "0")];
}

/** Compare two decimal strings: -1 (a<b), 0 (equal), 1 (a>b). */
export function compareDecimal(a: string, b: string): number {
  const pa = split(a);
  const pb = split(b);

  if (pa.negative !== pb.negative) return pa.negative ? -1 : 1;

  const sign = pa.negative ? -1 : 1;

  if (pa.int.length !== pb.int.length) {
    return (pa.int.length < pb.int.length ? -1 : 1) * sign;
  }
  if (pa.int !== pb.int) return (pa.int < pb.int ? -1 : 1) * sign;

  const [fa, fb] = padEndToLength(pa.frac, pb.frac);
  if (fa === fb) return 0;
  return (fa < fb ? -1 : 1) * sign;
}

/** Compare a decimal string against an integer threshold (safe for whole-number cutoffs). */
export function compareDecimalToNumber(a: string, n: number): number {
  return compareDecimal(a, String(n));
}

export interface DecimalFormatOptions {
  locale: string;
  minFractionDigits: number;
  maxFractionDigits: number;
}

function incrementWidth(digits: string): string | null {
  const arr = digits.split("");
  for (let i = arr.length - 1; i >= 0; i--) {
    if (arr[i] !== "9") {
      arr[i] = String(Number(arr[i]) + 1);
      return arr.join("");
    }
    arr[i] = "0";
  }
  return null; // overflow
}

/** Round the fraction to `max` digits, half-up, carrying into the integer part. */
function round(int: string, frac: string, max: number): [string, string] {
  if (frac.length <= max) return [int, frac];
  const keep = frac.slice(0, max);
  const next = frac[max];
  if (next < "5") return [int, keep];
  const bumped = incrementWidth(keep);
  if (bumped !== null) return [int, bumped];
  const bumpedInt = (BigInt(int) + 1n).toString();
  return [bumpedInt, "0".repeat(max)];
}

const groupCache = new Map<string, Intl.NumberFormat>();
const decimalSepCache = new Map<string, string>();

function groupInteger(intDigits: string, locale: string): string {
  let nf = groupCache.get(locale);
  if (!nf) {
    nf = new Intl.NumberFormat(locale, { useGrouping: true });
    groupCache.set(locale, nf);
  }
  return nf.format(BigInt(intDigits));
}

function decimalSeparator(locale: string): string {
  let sep = decimalSepCache.get(locale);
  if (sep === undefined) {
    sep =
      new Intl.NumberFormat(locale)
        .formatToParts(0.1)
        .find((p) => p.type === "decimal")?.value ?? ".";
    decimalSepCache.set(locale, sep);
  }
  return sep;
}

/** Format a decimal string with locale grouping and fixed fraction digits. */
export function formatDecimal(s: string, opts: DecimalFormatOptions): string {
  const parts = split(s);
  const [int, frac] = round(parts.int, parts.frac, opts.maxFractionDigits);
  const fracTrimmed = frac.replace(/0+$/, "");
  const fracPadded = fracTrimmed.padEnd(opts.minFractionDigits, "0");
  const grouped = groupInteger(int, opts.locale);
  const sep = decimalSeparator(opts.locale);
  const sign = parts.negative && !(int === "0" && fracPadded.replace(/0+$/, "") === "") ? "-" : "";
  return `${sign}${grouped}${sep}${fracPadded}`;
}
