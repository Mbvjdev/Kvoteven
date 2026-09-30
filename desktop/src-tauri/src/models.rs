//! Frozen, UI-facing domain model for the Kvoteven desktop backend.
//!
//! Everything here is a pure value: no I/O, no credentials, no process or
//! network access. Provider wire formats are parsed in `providers`; this
//! module only defines the types the frontend receives and the mood math.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use rust_decimal::Decimal;

/// The two account meters Kvoteven renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    #[serde(rename = "deepseek")]
    Deepseek,
    #[serde(rename = "openai-codex")]
    OpenaiCodex,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Deepseek => "deepseek",
            Provider::OpenaiCodex => "openai-codex",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which installed backend serves the Codex quota meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexSource {
    Auto,
    Codex,
    Hermes,
}

impl CodexSource {
    pub fn as_str(self) -> &'static str {
        match self {
            CodexSource::Auto => "auto",
            CodexSource::Codex => "codex",
            CodexSource::Hermes => "hermes",
        }
    }
}

impl fmt::Display for CodexSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The pet's mood, derived from the active meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mood {
    Happy,
    Steady,
    Sleepy,
    Asleep,
    Unknown,
}

/// The normalized, UI-ready result of a provider read.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Snapshot {
    pub provider: Provider,
    pub source: String,
    /// ISO-8601 (RFC 3339) timestamp of when the reading was taken.
    pub fetched_at: String,
    pub mood: Mood,
    /// Codex: weekly percent *remaining* (100 - used). `None` for DeepSeek.
    pub remaining_percent: Option<f64>,
    /// Codex: ISO-8601 reset instant. `None` for DeepSeek.
    pub resets_at: Option<String>,
    /// DeepSeek: preferred wallet currency (USD first). `None` for Codex.
    pub currency: Option<String>,
    /// DeepSeek: preferred wallet balance as a decimal string. `None` for Codex.
    pub balance: Option<String>,
    /// DeepSeek: net change since the first sight of this currency. `None` on
    /// first read, on currency change, and for Codex.
    pub net_change: Option<String>,
    /// DeepSeek: whether the API is available. `None` for Codex.
    pub is_available: Option<bool>,
}

/// Lightweight readiness summary; safe to compute without network or
/// credentials (and, in demo mode, without even probing the environment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AppStatus {
    pub demo: bool,
    pub version: String,
    pub deepseek_configured: bool,
    pub codex_available: bool,
    pub hermes_available: bool,
    pub platform: String,
}

/// A fixed, sanitized error. The `code` field is always one of the catalog in
/// [`AppError`]'s associated constants; raw response bodies, process output,
/// auth material and error details are never placed here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppError {
    pub code: String,
}

impl AppError {
    pub const DEMO_REJECTED: &'static str = "demo_mode_rejected";
    pub const INVALID_KEY: &'static str = "invalid_key";
    pub const KEYRING_UNAVAILABLE: &'static str = "keyring_unavailable";
    pub const KEYRING_WRITE_FAILED: &'static str = "keyring_write_failed";
    pub const KEYRING_DELETE_FAILED: &'static str = "keyring_delete_failed";
    pub const NOT_CONFIGURED: &'static str = "not_configured";
    pub const DEEPSEEK_REQUEST_FAILED: &'static str = "deepseek_request_failed";
    pub const DEEPSEEK_BAD_RESPONSE: &'static str = "deepseek_bad_response";
    pub const CODEX_UNAVAILABLE: &'static str = "codex_unavailable";
    pub const CODEX_REQUEST_FAILED: &'static str = "codex_request_failed";
    pub const CODEX_BAD_RESPONSE: &'static str = "codex_bad_response";
    pub const HERMES_UNAVAILABLE: &'static str = "hermes_unavailable";
    pub const HERMES_REQUEST_FAILED: &'static str = "hermes_request_failed";
    pub const HERMES_BAD_RESPONSE: &'static str = "hermes_bad_response";
    pub const TIMEOUT: &'static str = "timeout";
    pub const BUSY: &'static str = "busy";
    pub const INTERNAL: &'static str = "internal";

    /// Construct an error from one of the fixed catalog codes.
    pub fn new(code: &'static str) -> Self {
        AppError {
            code: code.to_string(),
        }
    }

    pub fn is(&self, code: &str) -> bool {
        self.code == code
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.code)
    }
}

impl std::error::Error for AppError {}

/// Kvoteven's app version, sourced from `Cargo.toml` so it can never drift from
/// the packaged artifact (also mirrored in the Codex `clientInfo` handshake).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// DeepSeek mood from an account balance (money thresholds).
pub fn deepseek_mood(total: Decimal) -> Mood {
    if total > Decimal::from(20) {
        Mood::Happy
    } else if total >= Decimal::from(5) {
        Mood::Steady
    } else if total > Decimal::from(0) {
        Mood::Sleepy
    } else {
        Mood::Asleep
    }
}

/// Codex mood from weekly percent *remaining*.
pub fn codex_mood(remaining: f64) -> Mood {
    if remaining > 50.0 {
        Mood::Happy
    } else if remaining > 20.0 {
        Mood::Steady
    } else if remaining > 0.0 {
        Mood::Sleepy
    } else {
        Mood::Asleep
    }
}

/// Strict decimal string parser matching `^-?[0-9]+(?:\.[0-9]+)?$`, at most 38
/// characters, with no exponent/whitespace/commas. Returns `None` unless the
/// text is exactly that shape and fits `rust_decimal`.
pub fn parse_strict_decimal(text: &str) -> Option<Decimal> {
    if !strict_decimal_shape(text) {
        return None;
    }
    Decimal::from_str(text).ok()
}

fn strict_decimal_shape(text: &str) -> bool {
    let bytes = text.as_bytes();
    let n = bytes.len();
    if n == 0 || n > 38 {
        return false;
    }
    let mut i = 0;
    if bytes[0] == b'-' {
        i += 1;
        if i == n {
            return false;
        }
    }
    let mut int_digits = 0usize;
    let mut seen_dot = false;
    let mut frac_digits = 0usize;
    while i < n {
        match bytes[i] {
            b'0'..=b'9' => {
                if seen_dot {
                    frac_digits += 1;
                } else {
                    int_digits += 1;
                }
            }
            b'.' => {
                if seen_dot {
                    return false;
                }
                seen_dot = true;
            }
            _ => return false,
        }
        i += 1;
    }
    int_digits >= 1 && (!seen_dot || frac_digits >= 1)
}

/// Format a `Decimal` as its exact canonical string ("42.50", "0.001", "-0.03").
///
/// No rounding is applied here on purpose: the exact value must survive intact
/// to the display layer, which does any money formatting (e.g. two decimal
/// places) it needs. Rounding in the backend would collapse a tiny positive
/// balance such as `0.001` into "0.00" and let a downstream consumer
/// misclassify the account as empty (`Asleep`) when the backend computed
/// `Sleepy` from the true value.
pub fn format_money(value: Decimal) -> String {
    value.to_string()
}

/// True when a snapshot is still usable: its `fetched_at` falls inside the
/// accepted freshness window and its `resets_at` (when present) is in the
/// future. Used to decide whether a cached snapshot may be served instead of
/// re-reading the provider, so stale or expired data is never re-served.
pub fn snapshot_fresh(s: &Snapshot) -> bool {
    let now = Utc::now();
    let Ok(fetched) = DateTime::parse_from_rfc3339(&s.fetched_at) else {
        return false;
    };
    if !(-60..=600).contains(&(now - fetched.with_timezone(&Utc)).num_seconds()) {
        return false;
    }
    match &s.resets_at {
        Some(r) => DateTime::parse_from_rfc3339(r)
            .map(|t| t > now)
            .unwrap_or(false),
        None => true,
    }
}

/// ISO-8601 / RFC 3339 string with a `Z` suffix and second precision.
pub fn iso8601(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// ISO-8601 / RFC 3339 string for a UNIX-seconds instant.
pub fn iso8601_from_unix(secs: i64) -> Option<String> {
    DateTime::from_timestamp(secs, 0).map(iso8601)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;

    fn dec(s: &str) -> Decimal {
        s.parse::<Decimal>().unwrap()
    }

    #[test]
    fn provider_serializes_to_wire_names() {
        assert_eq!(
            serde_json::to_string(&Provider::Deepseek).unwrap(),
            "\"deepseek\""
        );
        assert_eq!(
            serde_json::to_string(&Provider::OpenaiCodex).unwrap(),
            "\"openai-codex\""
        );
        assert_eq!(
            serde_json::from_str::<Provider>("\"deepseek\"").unwrap(),
            Provider::Deepseek
        );
        assert_eq!(
            serde_json::from_str::<Provider>("\"openai-codex\"").unwrap(),
            Provider::OpenaiCodex
        );
        assert!(serde_json::from_str::<Provider>("\"anthropic\"").is_err());
    }

    #[test]
    fn codex_source_round_trips() {
        for (s, expected) in [
            ("\"auto\"", CodexSource::Auto),
            ("\"codex\"", CodexSource::Codex),
            ("\"hermes\"", CodexSource::Hermes),
        ] {
            assert_eq!(serde_json::from_str::<CodexSource>(s).unwrap(), expected);
            assert_eq!(serde_json::to_string(&expected).unwrap(), s);
        }
    }

    #[test]
    fn mood_round_trips() {
        for (s, expected) in [
            ("\"happy\"", Mood::Happy),
            ("\"steady\"", Mood::Steady),
            ("\"sleepy\"", Mood::Sleepy),
            ("\"asleep\"", Mood::Asleep),
            ("\"unknown\"", Mood::Unknown),
        ] {
            assert_eq!(serde_json::from_str::<Mood>(s).unwrap(), expected);
            assert_eq!(serde_json::to_string(&expected).unwrap(), s);
        }
    }

    #[test]
    fn snapshot_serializes_with_snake_case_keys() {
        let snap = Snapshot {
            provider: Provider::OpenaiCodex,
            source: "codex_app_server".to_string(),
            fetched_at: "2026-09-29T19:05:02Z".to_string(),
            mood: Mood::Happy,
            remaining_percent: Some(68.0),
            resets_at: Some("2026-10-05T19:05:02Z".to_string()),
            currency: None,
            balance: None,
            net_change: None,
            is_available: None,
        };
        let v = serde_json::to_value(&snap).unwrap();
        assert_eq!(v["provider"], "openai-codex");
        assert_eq!(v["source"], "codex_app_server");
        assert_eq!(v["fetched_at"], "2026-09-29T19:05:02Z");
        assert_eq!(v["mood"], "happy");
        assert_eq!(v["remaining_percent"], 68.0);
        assert_eq!(v["resets_at"], "2026-10-05T19:05:02Z");
        assert!(v.get("is_available").unwrap().is_null());
    }

    #[test]
    fn app_status_serializes_fields() {
        let st = AppStatus {
            demo: false,
            version: "0.4.0".to_string(),
            deepseek_configured: true,
            codex_available: false,
            hermes_available: true,
            platform: "macos".to_string(),
        };
        let v = serde_json::to_value(&st).unwrap();
        assert_eq!(v["demo"], false);
        assert_eq!(v["version"], "0.4.0");
        assert_eq!(v["deepseek_configured"], true);
        assert_eq!(v["codex_available"], false);
        assert_eq!(v["hermes_available"], true);
        assert_eq!(v["platform"], "macos");
    }

    #[test]
    fn app_error_is_struct_literal_compatible_and_displayable() {
        let e = AppError {
            code: "timeout".to_string(),
        };
        assert_eq!(serde_json::to_string(&e).unwrap(), "{\"code\":\"timeout\"}");
        assert_eq!(format!("{e}"), "timeout");
        assert!(e.is(AppError::TIMEOUT));
    }

    #[test]
    fn deepseek_mood_thresholds() {
        // >20 happy, >=5 steady, >0 sleepy, <=0 asleep.
        assert_eq!(deepseek_mood(dec("88.00")), Mood::Happy);
        assert_eq!(deepseek_mood(dec("20.01")), Mood::Happy);
        assert_eq!(deepseek_mood(dec("20")), Mood::Steady);
        assert_eq!(deepseek_mood(dec("5")), Mood::Steady);
        assert_eq!(deepseek_mood(dec("4.99")), Mood::Sleepy);
        assert_eq!(deepseek_mood(dec("0.01")), Mood::Sleepy);
        assert_eq!(deepseek_mood(dec("0")), Mood::Asleep);
        assert_eq!(deepseek_mood(dec("-0.01")), Mood::Asleep);
    }

    #[test]
    fn codex_mood_thresholds() {
        // remaining >50 happy, >20 steady, >0 sleepy, else asleep.
        assert_eq!(codex_mood(100.0), Mood::Happy);
        assert_eq!(codex_mood(50.1), Mood::Happy);
        assert_eq!(codex_mood(50.0), Mood::Steady);
        assert_eq!(codex_mood(20.1), Mood::Steady);
        assert_eq!(codex_mood(20.0), Mood::Sleepy);
        assert_eq!(codex_mood(0.1), Mood::Sleepy);
        assert_eq!(codex_mood(0.0), Mood::Asleep);
        assert_eq!(codex_mood(-1.0), Mood::Asleep);
    }

    #[test]
    fn strict_decimal_accepts_and_rejects_correctly() {
        for ok in [
            "0",
            "-0",
            "42.50",
            "42.5",
            "110",
            "-0.03",
            "99999999999999999999",
        ] {
            assert!(parse_strict_decimal(ok).is_some(), "should accept {ok}");
        }
        for bad in [
            "", "NaN", "Infinity", "1.2junk", "1,25", "1e3", ".5", "5.", "+5", " 5", "5 ", "-",
        ] {
            assert!(parse_strict_decimal(bad).is_none(), "should reject {bad}");
        }
        // 38 chars but overflows rust_decimal precision -> rejected.
        let huge = "9".repeat(38);
        assert!(parse_strict_decimal(&huge).is_none());
    }

    #[test]
    fn money_is_exact_canonical_string_no_rounding() {
        assert_eq!(format_money(dec("42.5")), "42.5");
        assert_eq!(format_money(dec("42.50")), "42.50");
        assert_eq!(format_money(dec("42")), "42");
        assert_eq!(format_money(dec("-0.03")), "-0.03");
        assert_eq!(format_money(dec("42.375")), "42.375");
    }

    #[test]
    fn tiny_positive_money_is_not_rounded_away() {
        // 0.001 must survive as "0.001" (and read as Sleepy), never "0.00"
        // (which the frontend would classify as Asleep).
        assert_eq!(format_money(dec("0.001")), "0.001");
        assert_eq!(deepseek_mood(dec("0.001")), Mood::Sleepy);
        assert_eq!(deepseek_mood(Decimal::ZERO), Mood::Asleep);
    }

    #[test]
    fn huge_money_sums_round_trip_exactly() {
        let max = "79228162514264337593543950335";
        assert_eq!(format_money(dec(max)), max);
        assert_eq!(
            format_money(dec("9999999999999999.99")),
            "9999999999999999.99"
        );
    }

    #[test]
    fn app_version_comes_from_cargo_manifest() {
        assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn snapshot_freshness_window_and_reset() {
        let base = Snapshot {
            provider: Provider::Deepseek,
            source: "x".to_string(),
            fetched_at: iso8601(Utc::now()),
            mood: Mood::Happy,
            remaining_percent: None,
            resets_at: None,
            currency: None,
            balance: None,
            net_change: None,
            is_available: None,
        };
        // Fresh now, no reset -> fresh.
        assert!(snapshot_fresh(&base));

        // Stale fetched_at (2 hours old) -> not fresh.
        let stale = Snapshot {
            fetched_at: iso8601(Utc::now() - chrono::Duration::seconds(7200)),
            ..base.clone()
        };
        assert!(!snapshot_fresh(&stale));

        // Reset already passed -> not fresh.
        let reset_passed = Snapshot {
            resets_at: Some(iso8601(Utc::now() - chrono::Duration::seconds(60))),
            ..base.clone()
        };
        assert!(!snapshot_fresh(&reset_passed));

        // Reset in the future -> fresh.
        let reset_future = Snapshot {
            resets_at: Some(iso8601(Utc::now() + chrono::Duration::seconds(3600))),
            ..base
        };
        assert!(snapshot_fresh(&reset_future));
    }

    #[test]
    fn iso8601_helpers() {
        let dt = DateTime::parse_from_rfc3339("2026-09-29T19:05:02Z").unwrap();
        let utc = dt.with_timezone(&Utc);
        assert_eq!(iso8601(utc), "2026-09-29T19:05:02Z");
        assert_eq!(
            iso8601_from_unix(utc.timestamp()),
            Some("2026-09-29T19:05:02Z".to_string())
        );
        assert_eq!(iso8601_from_unix(i64::MIN), None);
    }
}
