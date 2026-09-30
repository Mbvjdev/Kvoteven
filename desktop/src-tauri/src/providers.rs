//! Provider adapters: DeepSeek balance (HTTPS), Codex quota (app-server over
//! stdio JSON-RPC) and Hermes quota (CLI). All network/process details are
//! confined here; wire formats are validated strictly and errors are mapped to
//! fixed `AppError` codes.

use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;

use crate::models::{parse_strict_decimal, AppError, APP_VERSION};
use crate::runner::{run_bounded, Executable, RunError, SpawnOptions, SpawnedProcess};

pub const DEEPSEEK_BALANCE_URL: &str = "https://api.deepseek.com/user/balance";

/// Exact, read-only Codex usage argv (see HermesQuotaReader in the macOS app).
pub const HERMES_ARGV: [&str; 6] = [
    "--profile",
    "default",
    "usage",
    "--provider",
    "openai-codex",
    "--json",
];

/// The `codex app-server` subcommand argument.
pub const CODEX_APP_SERVER_ARG: &str = "app-server";

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_NOTIFICATIONS: usize = 4096;
const CODEX_TIMEOUT: Duration = Duration::from_secs(25);
const HERMES_TIMEOUT: Duration = Duration::from_secs(25);
const DEEPSEEK_TIMEOUT: Duration = Duration::from_secs(15);

/// A validated DeepSeek wallet.
#[derive(Debug, Clone, PartialEq)]
pub struct Wallet {
    pub currency: String,
    pub total: Decimal,
}

/// Result of a DeepSeek balance read.
#[derive(Debug, Clone, PartialEq)]
pub struct DeepSeekBalance {
    pub is_available: bool,
    pub wallets: Vec<Wallet>,
}

/// Result of a percent-based (Codex or Hermes) quota read.
#[derive(Debug, Clone, PartialEq)]
pub struct PercentQuota {
    /// The source label to place on the snapshot.
    pub source: String,
    /// Weekly percent *used*; `None` when no weekly window was reported.
    pub used_percent: Option<f64>,
    pub resets_at: Option<DateTime<Utc>>,
    pub fetched_at: DateTime<Utc>,
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Backend abstraction so `service` is testable without network or processes.
pub trait ProviderBackend: Send + Sync {
    fn read_deepseek<'a>(
        &'a self,
        key: &'a str,
    ) -> BoxFuture<'a, Result<DeepSeekBalance, AppError>>;
    fn read_codex(&self) -> BoxFuture<'static, Result<PercentQuota, AppError>>;
    fn read_hermes(&self) -> BoxFuture<'static, Result<PercentQuota, AppError>>;
    fn codex_available(&self) -> bool;
    fn hermes_available(&self) -> bool;
}

/// Real backend: direct HTTPS to DeepSeek, `codex app-server` and the `hermes`
/// CLI for Codex quota.
pub struct RealBackend {
    http: reqwest::Client,
}

impl RealBackend {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(DEEPSEEK_TIMEOUT)
            .build()
            .expect("static reqwest client configuration must build");
        RealBackend { http }
    }
}

impl Default for RealBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderBackend for RealBackend {
    fn read_deepseek<'a>(
        &'a self,
        key: &'a str,
    ) -> BoxFuture<'a, Result<DeepSeekBalance, AppError>> {
        Box::pin(fetch_deepseek_balance(
            &self.http,
            key,
            DEEPSEEK_BALANCE_URL,
        ))
    }

    fn read_codex(&self) -> BoxFuture<'static, Result<PercentQuota, AppError>> {
        Box::pin(async move {
            let exec =
                discover_codex().ok_or_else(|| AppError::new(AppError::CODEX_UNAVAILABLE))?;
            read_codex_quota(&exec).await
        })
    }

    fn read_hermes(&self) -> BoxFuture<'static, Result<PercentQuota, AppError>> {
        Box::pin(async move {
            let exec =
                discover_hermes().ok_or_else(|| AppError::new(AppError::HERMES_UNAVAILABLE))?;
            read_hermes_quota(&exec).await
        })
    }

    fn codex_available(&self) -> bool {
        discover_codex().is_some()
    }

    fn hermes_available(&self) -> bool {
        discover_hermes().is_some()
    }
}

fn map_run_error_codex(e: RunError) -> AppError {
    match e {
        RunError::TimedOut => AppError::new(AppError::TIMEOUT),
        RunError::OutputTooLarge => AppError::new(AppError::CODEX_BAD_RESPONSE),
        RunError::Io(_) | RunError::ExitStatus(_) => AppError::new(AppError::CODEX_REQUEST_FAILED),
    }
}

fn map_run_error_hermes(e: RunError) -> AppError {
    match e {
        RunError::TimedOut => AppError::new(AppError::TIMEOUT),
        RunError::OutputTooLarge => AppError::new(AppError::HERMES_BAD_RESPONSE),
        RunError::Io(_) | RunError::ExitStatus(_) => AppError::new(AppError::HERMES_REQUEST_FAILED),
    }
}

// ---------------------------------------------------------------------------
// DeepSeek
// ---------------------------------------------------------------------------

/// Perform the documented DeepSeek balance GET and parse it strictly.
pub async fn fetch_deepseek_balance(
    client: &reqwest::Client,
    key: &str,
    url: &str,
) -> Result<DeepSeekBalance, AppError> {
    let auth = zeroize::Zeroizing::new(format!("Bearer {}", key));
    let mut response = client
        .get(url)
        .header(reqwest::header::AUTHORIZATION, auth.as_str())
        .send()
        .await
        .map_err(deepseek_http_error)?;

    if !response.status().is_success() {
        return Err(AppError::new(AppError::DEEPSEEK_REQUEST_FAILED));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(deepseek_http_error)? {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(AppError::new(AppError::DEEPSEEK_BAD_RESPONSE));
        }
        body.extend_from_slice(&chunk);
    }

    parse_deepseek_body(&body)
}

/// Map a DeepSeek transport error to a fixed code, keeping timeouts distinct.
fn deepseek_http_error(e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::new(AppError::TIMEOUT)
    } else {
        AppError::new(AppError::DEEPSEEK_REQUEST_FAILED)
    }
}

#[derive(Deserialize)]
struct DeepSeekRaw {
    #[serde(rename = "is_available")]
    is_available: bool,
    #[serde(rename = "balance_infos")]
    balance_infos: Vec<DeepSeekWalletRaw>,
}

#[derive(Deserialize)]
struct DeepSeekWalletRaw {
    currency: String,
    #[serde(rename = "total_balance")]
    total_balance: String,
}

/// Parse and validate a DeepSeek balance body: USD/CNY only, no duplicate
/// currencies, non-empty, strict decimal amounts.
pub fn parse_deepseek_body(body: &[u8]) -> Result<DeepSeekBalance, AppError> {
    let raw: DeepSeekRaw =
        serde_json::from_slice(body).map_err(|_| AppError::new(AppError::DEEPSEEK_BAD_RESPONSE))?;
    if raw.balance_infos.is_empty() {
        return Err(AppError::new(AppError::DEEPSEEK_BAD_RESPONSE));
    }
    let mut seen = HashSet::new();
    let mut wallets = Vec::with_capacity(raw.balance_infos.len());
    for w in raw.balance_infos {
        if w.currency != "USD" && w.currency != "CNY" {
            return Err(AppError::new(AppError::DEEPSEEK_BAD_RESPONSE));
        }
        if !seen.insert(w.currency.clone()) {
            // Duplicate currency wallets are rejected.
            return Err(AppError::new(AppError::DEEPSEEK_BAD_RESPONSE));
        }
        let total = parse_strict_decimal(&w.total_balance)
            .ok_or_else(|| AppError::new(AppError::DEEPSEEK_BAD_RESPONSE))?;
        wallets.push(Wallet {
            currency: w.currency,
            total,
        });
    }
    Ok(DeepSeekBalance {
        is_available: raw.is_available,
        wallets,
    })
}

// ---------------------------------------------------------------------------
// Codex app-server (JSON-RPC over stdio)
// ---------------------------------------------------------------------------

/// Read the Codex quota from `codex app-server`, returning the raw `result`
/// body of the `account/rateLimits/read` response.
///
/// The official app-server is plain JSON-RPC over stdio, *not* MCP: the
/// handshake is `initialize` (with only `clientInfo`, no `protocolVersion`,
/// no `capabilities`), then the `initialized` notification, then
/// `account/rateLimits/read`. An `error` on `initialize` aborts before any
/// further message is sent.
pub async fn codex_protocol(
    exec: &Executable,
    args: &[String],
    timeout: Duration,
) -> Result<Value, RunError> {
    let mut child = SpawnedProcess::spawn(exec, args)?;

    let exchange = async {
        let mut budget = MAX_RESPONSE_BYTES;

        let init = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": { "name": "kvoteven", "title": "Kvoteven", "version": APP_VERSION }
            }
        });
        child.write_line(&init.to_string()).await?;

        // Await the initialize result/error and check it before continuing.
        let init_resp = read_until_response(&mut child, &mut budget, 1).await?;
        if init_resp.get("error").is_some() {
            return Err(RunError::Io(io_error(
                "codex app-server rejected initialize",
            )));
        }
        if init_resp.get("result").is_none() {
            return Err(RunError::Io(io_error(
                "codex app-server initialize response has no result",
            )));
        }

        // The `initialized` notification (not MCP's `notifications/initialized`).
        child
            .write_line(r#"{"jsonrpc":"2.0","method":"initialized"}"#)
            .await?;

        let read = serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "account/rateLimits/read", "params": {}
        });
        child.write_line(&read.to_string()).await?;
        let resp = read_until_response(&mut child, &mut budget, 2).await?;
        if resp.get("error").is_some() {
            return Err(RunError::Io(io_error(
                "codex app-server returned an error for account/rateLimits/read",
            )));
        }
        let result = resp
            .get("result")
            .cloned()
            .ok_or_else(|| RunError::Io(io_error("codex app-server response has no result")))?;
        Ok::<Value, RunError>(result)
    };

    let result = tokio::time::timeout(timeout, exchange).await;
    child.reap().await;
    match result {
        Err(_elapsed) => Err(RunError::TimedOut),
        Ok(Err(e)) => Err(e),
        Ok(Ok(v)) => Ok(v),
    }
}

fn io_error(msg: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg)
}

async fn read_until_response(
    child: &mut SpawnedProcess,
    budget: &mut usize,
    target_id: u64,
) -> Result<Value, RunError> {
    let mut notifications = 0usize;
    loop {
        let line = match child.read_line(budget).await? {
            Some(l) => l,
            None => {
                return Err(RunError::Io(io_error(
                    "codex app-server closed before responding",
                )))
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = serde_json::from_str(&line)
            .map_err(|_| RunError::Io(io_error("non-JSON line from codex app-server")))?;
        match msg.get("id").and_then(Value::as_u64) {
            Some(id) if id == target_id => {
                if msg.get("result").is_some() || msg.get("error").is_some() {
                    return Ok(msg);
                }
                // A request from the server colliding with our id: reject.
                return Err(RunError::Io(io_error("unexpected request from server")));
            }
            Some(_) => {
                // A server-initiated request (method + id) is never expected
                // from the app-server; a mismatched/duplicate *response* id is
                // skipped so it cannot spoof the awaited result.
                if msg.get("method").is_some() {
                    return Err(RunError::Io(io_error("unexpected request from server")));
                }
            }
            None => {
                notifications += 1;
                if notifications > MAX_NOTIFICATIONS {
                    return Err(RunError::OutputTooLarge);
                }
            }
        }
    }
}

/// Read the Codex quota from `codex app-server` and normalize to a weekly
/// percent.
pub async fn read_codex_quota(exec: &Executable) -> Result<PercentQuota, AppError> {
    let args = vec![CODEX_APP_SERVER_ARG.to_string()];
    let result = codex_protocol(exec, &args, CODEX_TIMEOUT)
        .await
        .map_err(map_run_error_codex)?;
    let (used, resets) = parse_codex_rate_limits(&result)?;
    Ok(PercentQuota {
        source: "codex_app_server".to_string(),
        used_percent: used,
        resets_at: resets,
        fetched_at: Utc::now(),
    })
}

struct RateLimitLeaf {
    used_percent: Option<f64>,
    window_duration_mins: Option<i64>,
    resets_at: Option<i64>,
}

/// Parse the `account/rateLimits/read` result.
///
/// `rateLimitsByLimitId.codex` is preferred and fail-closed when the map lacks
/// a `codex` key. The legacy `rateLimits` array is only consulted with
/// correct/absent `limitId`. The weekly window is the sole
/// `windowDurationMins == 10080` leaf; its `usedPercent` must be a finite
/// number in 0..=100. A missing weekly window yields `None` (no invented
/// number), never a hard error.
pub fn parse_codex_rate_limits(
    result: &Value,
) -> Result<(Option<f64>, Option<DateTime<Utc>>), AppError> {
    if let Some(map) = result.get("rateLimitsByLimitId").and_then(Value::as_object) {
        let codex = map
            .get("codex")
            .ok_or_else(|| AppError::new(AppError::CODEX_BAD_RESPONSE))?;
        let leaves = collect_rate_limits(codex);
        return weekly_from_leaves(leaves);
    }
    if let Some(arr) = result.get("rateLimits").and_then(Value::as_array) {
        let mut leaves = Vec::new();
        for item in arr {
            // Legacy entries are allowed only with correct/absent limitId.
            if let Some(id) = item.get("limitId").and_then(Value::as_str) {
                if id != "codex" {
                    continue;
                }
            }
            leaves.extend(collect_rate_limits(item));
        }
        return weekly_from_leaves(leaves);
    }
    Err(AppError::new(AppError::CODEX_BAD_RESPONSE))
}

fn collect_rate_limits(v: &Value) -> Vec<RateLimitLeaf> {
    let mut out = Vec::new();
    collect_rate_limits_inner(v, &mut out);
    out
}

fn collect_rate_limits_inner(v: &Value, out: &mut Vec<RateLimitLeaf>) {
    let Some(obj) = v.as_object() else { return };
    let mut has_children = false;
    for key in ["primary", "secondary"] {
        if let Some(child) = obj.get(key) {
            has_children = true;
            collect_rate_limits_inner(child, out);
        }
    }
    if !has_children {
        let used_percent = obj
            .get("usedPercent")
            .or_else(|| obj.get("used_percent"))
            .and_then(Value::as_f64);
        let window_duration_mins = obj
            .get("windowDurationMins")
            .or_else(|| obj.get("window_duration_mins"))
            .and_then(Value::as_i64);
        let resets_at = obj
            .get("resetsAt")
            .or_else(|| obj.get("resets_at"))
            .and_then(Value::as_i64);
        out.push(RateLimitLeaf {
            used_percent,
            window_duration_mins,
            resets_at,
        });
    }
}

fn weekly_from_leaves(
    leaves: Vec<RateLimitLeaf>,
) -> Result<(Option<f64>, Option<DateTime<Utc>>), AppError> {
    let weekly = leaves
        .into_iter()
        .find(|l| l.window_duration_mins == Some(10080));
    match weekly {
        None => Ok((None, None)),
        Some(l) => {
            let used = l
                .used_percent
                .ok_or_else(|| AppError::new(AppError::CODEX_BAD_RESPONSE))?;
            if !used.is_finite() || !(0.0..=100.0).contains(&used) {
                return Err(AppError::new(AppError::CODEX_BAD_RESPONSE));
            }
            let resets = l
                .resets_at
                .and_then(|secs| DateTime::from_timestamp(secs, 0));
            Ok((Some(used), resets))
        }
    }
}

// ---------------------------------------------------------------------------
// Hermes CLI
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct HermesRaw {
    provider: String,
    source: String,
    #[serde(rename = "fetched_at")]
    fetched_at: String,
    #[serde(rename = "unavailable_reason", default)]
    unavailable_reason: Option<String>,
    windows: Vec<HermesWindow>,
}

#[derive(Deserialize)]
struct HermesWindow {
    label: String,
    #[serde(rename = "used_percent")]
    used_percent: Option<f64>,
    #[serde(rename = "resets_at")]
    resets_at: Option<String>,
}

/// Read Codex quota from the `hermes` CLI and normalize to a weekly percent.
pub async fn read_hermes_quota(exec: &Executable) -> Result<PercentQuota, AppError> {
    let args: Vec<String> = HERMES_ARGV.iter().map(|s| s.to_string()).collect();
    let opts = SpawnOptions {
        timeout: HERMES_TIMEOUT,
        max_output_bytes: MAX_RESPONSE_BYTES,
    };
    let out = run_bounded(exec, &args, &opts)
        .await
        .map_err(map_run_error_hermes)?;
    parse_hermes_quota(&out)
}

/// Parse and validate a Hermes `usage --json` body: provider must be
/// `openai-codex`, `unavailable_reason` must be absent, the fetched timestamp
/// must parse, and the weekly window's `used_percent` must be in 0..=100.
pub fn parse_hermes_quota(body: &[u8]) -> Result<PercentQuota, AppError> {
    let raw: HermesRaw =
        serde_json::from_slice(body).map_err(|_| AppError::new(AppError::HERMES_BAD_RESPONSE))?;
    if raw.provider != "openai-codex" {
        return Err(AppError::new(AppError::HERMES_BAD_RESPONSE));
    }
    if raw.source.is_empty() {
        return Err(AppError::new(AppError::HERMES_BAD_RESPONSE));
    }
    if raw.unavailable_reason.is_some() {
        return Err(AppError::new(AppError::HERMES_BAD_RESPONSE));
    }
    let fetched_at = DateTime::parse_from_rfc3339(&raw.fetched_at)
        .map_err(|_| AppError::new(AppError::HERMES_BAD_RESPONSE))?;

    let weekly = raw
        .windows
        .into_iter()
        .find(|w| w.label.eq_ignore_ascii_case("weekly"));
    let (used, resets) = match weekly {
        None => (None, None),
        Some(w) => {
            let used = w
                .used_percent
                .ok_or_else(|| AppError::new(AppError::HERMES_BAD_RESPONSE))?;
            if !used.is_finite() || !(0.0..=100.0).contains(&used) {
                return Err(AppError::new(AppError::HERMES_BAD_RESPONSE));
            }
            let resets = match w.resets_at {
                Some(s) => {
                    let dt = DateTime::parse_from_rfc3339(&s)
                        .map_err(|_| AppError::new(AppError::HERMES_BAD_RESPONSE))?;
                    Some(dt.with_timezone(&Utc))
                }
                None => None,
            };
            (Some(used), resets)
        }
    };

    Ok(PercentQuota {
        source: raw.source,
        used_percent: used,
        resets_at: resets,
        fetched_at: fetched_at.with_timezone(&Utc),
    })
}

// ---------------------------------------------------------------------------
// Executable discovery
// ---------------------------------------------------------------------------

/// The official `@openai/codex` npm package ships a native vendor binary under
/// each platform-specific optional package: `@openai/codex-<platform>/vendor/
/// <triple>/bin/codex[.exe]`. The `bin/codex.js` entry and the `codex` PATH
/// shim are shell/node wrappers that spawn the vendor binary as a child, so
/// invoking them leaves a parent chain that a plain `kill` cannot reap. We
/// therefore prefer the native vendor binary directly.
pub fn discover_codex() -> Option<Executable> {
    // 1. Explicit native-binary override (used by the CLI check path).
    if let Ok(env) = std::env::var("KVOTEVEN_CODEX") {
        let p = PathBuf::from(&env);
        if p.is_absolute() && is_executable(&p) {
            return Some(Executable::Direct(p));
        }
    }
    // 2. Native vendor binary from a local npm install (walk node_modules).
    for root in node_modules_roots() {
        if let Some(exe) = find_codex_vendor_binary(&root) {
            return Some(exe);
        }
    }
    // 3. PATH fallback; on Windows reject shell shims (.cmd/.bat).
    which_executable("codex")
}

/// Return the `node_modules` roots to search for the Codex vendor binary: the
/// current directory and its ancestors (the same resolution node uses), so a
/// local `npm install @openai/codex` is found regardless of where the app runs.
fn node_modules_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        roots.push(d.join("node_modules"));
        dir = d.parent().map(Path::to_path_buf);
    }
    roots
}

/// Map the current platform to the `@openai/codex-<platform>` npm package
/// name, or `None` when the platform has no official optional package.
fn codex_platform_pkg() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("darwin-arm64"),
        ("macos", "x86_64") => Some("darwin-x64"),
        ("linux", "aarch64") => Some("linux-arm64"),
        ("linux", "x86_64") => Some("linux-x64"),
        ("windows", "aarch64") => Some("win32-arm64"),
        ("windows", "x86_64") => Some("win32-x64"),
        _ => None,
    }
}

/// Search a single `node_modules` root for the native Codex vendor binary,
/// tolerating any `vendor/<triple>` naming the package chose.
fn find_codex_vendor_binary(node_modules: &Path) -> Option<Executable> {
    let pkg = codex_platform_pkg()?;
    let vendor = node_modules
        .join(format!("@openai/codex-{pkg}"))
        .join("vendor");
    let entries = std::fs::read_dir(&vendor).ok()?;
    for entry in entries.flatten() {
        let bin = entry.path().join("bin").join(exe_name("codex"));
        if is_executable(&bin) {
            return Some(Executable::Direct(bin));
        }
    }
    None
}

/// `codex` on Windows, `codex.exe` elsewhere.
fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Discover the Hermes executable: an explicit `KVOTEVEN_HERMES` absolute path
/// wins, then fixed install locations, then `PATH`.
pub fn discover_hermes() -> Option<Executable> {
    if let Ok(env) = std::env::var("KVOTEVEN_HERMES") {
        let p = PathBuf::from(&env);
        if p.is_absolute() && is_executable(&p) {
            return Some(Executable::Direct(p));
        }
    }
    let home = dirs::home_dir()?;
    for p in [
        home.join(".local/bin/hermes"),
        home.join(".hermes/hermes-agent/.hermes/bin/hermes"),
    ] {
        if is_executable(&p) {
            return Some(Executable::Direct(p));
        }
    }
    which_executable("hermes")
}

/// `PATH` lookup that never resolves to a shell shim: on Windows a `.cmd` or
/// `.bat` wrapper is rejected so we only ever spawn the native binary.
fn which_executable(name: &str) -> Option<Executable> {
    let found = which::which(name).ok()?;
    if is_cmd_or_bat(&found) {
        return None;
    }
    Some(Executable::Direct(found))
}

/// True for Windows `.cmd`/`.bat` shims (which invoke `cmd.exe`, i.e. a shell).
fn is_cmd_or_bat(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    matches!(ext.as_deref(), Some("cmd") | Some("bat"))
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        // Only native binaries; `.cmd`/`.bat` are shell shims, never spawned.
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        matches!(ext.as_deref(), Some("exe"))
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Mood;
    use chrono::DateTime;
    use rust_decimal::Decimal;
    use serde_json::Value;

    fn dec(s: &str) -> Decimal {
        s.parse::<Decimal>().unwrap()
    }

    // --- DeepSeek parse ----------------------------------------------------

    fn deepseek_doc(total: &str, currency: &str, available: bool) -> String {
        format!(
            r#"{{"is_available":{available},"balance_infos":[{{"currency":"{currency}","total_balance":"{total}","granted_balance":"0.00","topped_up_balance":"{total}"}}]}}"#
        )
    }

    #[test]
    fn deepseek_parses_usd_and_cny() {
        let b = parse_deepseek_body(deepseek_doc("42.37", "USD", true).as_bytes()).unwrap();
        assert!(b.is_available);
        assert_eq!(
            b.wallets,
            vec![Wallet {
                currency: "USD".to_string(),
                total: dec("42.37")
            }]
        );
        let b = parse_deepseek_body(deepseek_doc("110", "CNY", false).as_bytes()).unwrap();
        assert!(!b.is_available);
        assert_eq!(b.wallets[0].total, dec("110"));
    }

    #[test]
    fn deepseek_rejects_bad_money_currency_and_duplicates() {
        for bad in ["NaN", "1.2junk", "Infinity", "", "1,25", "1e3", ".5", "5."] {
            assert_eq!(
                parse_deepseek_body(deepseek_doc(bad, "USD", true).as_bytes())
                    .unwrap_err()
                    .code,
                AppError::DEEPSEEK_BAD_RESPONSE,
                "should reject {bad}"
            );
        }
        // Unexpected currency.
        assert!(parse_deepseek_body(deepseek_doc("1.00", "EUR", true).as_bytes()).is_err());
        // Empty wallet list.
        assert!(parse_deepseek_body(br#"{"is_available":true,"balance_infos":[]}"#).is_err());
        // Duplicate currencies.
        let dup = br#"{"is_available":true,"balance_infos":[{"currency":"USD","total_balance":"1.00"},{"currency":"USD","total_balance":"2.00"}]}"#;
        assert_eq!(
            parse_deepseek_body(dup).unwrap_err().code,
            AppError::DEEPSEEK_BAD_RESPONSE
        );
    }

    // --- Codex rate-limits parse -------------------------------------------

    fn codex_map(used: f64, window_mins: i64, reset: i64) -> Value {
        serde_json::json!({
            "rateLimitsByLimitId": {
                "codex": {
                    "primary": { "usedPercent": used, "windowDurationMins": window_mins, "resetsAt": reset },
                    "secondary": { "usedPercent": 0.0, "windowDurationMins": window_mins, "resetsAt": reset }
                }
            }
        })
    }

    #[test]
    fn codex_prefers_map_and_picks_weekly_primary() {
        let reset = 2_000_000_000i64;
        let (used, resets) = parse_codex_rate_limits(&codex_map(32.0, 10080, reset)).unwrap();
        assert_eq!(used, Some(32.0));
        assert_eq!(resets, DateTime::from_timestamp(reset, 0));
    }

    #[test]
    fn codex_map_without_codex_key_fails_closed() {
        let v = serde_json::json!({"rateLimitsByLimitId": {"chatgpt": {"usedPercent": 1.0}}});
        assert_eq!(
            parse_codex_rate_limits(&v).unwrap_err().code,
            AppError::CODEX_BAD_RESPONSE
        );
    }

    #[test]
    fn codex_missing_weekly_yields_none_not_an_error() {
        let v = codex_map(10.0, 4320, 2_000_000_000);
        let (used, resets) = parse_codex_rate_limits(&v).unwrap();
        assert_eq!(used, None);
        assert_eq!(resets, None);
    }

    #[test]
    fn codex_out_of_range_percent_fails() {
        for bad in [-1.0, 100.5, f64::NAN, f64::INFINITY] {
            let v = codex_map(bad, 10080, 2_000_000_000);
            assert_eq!(
                parse_codex_rate_limits(&v).unwrap_err().code,
                AppError::CODEX_BAD_RESPONSE,
                "should reject {bad}"
            );
        }
    }

    #[test]
    fn codex_legacy_rate_limits_allow_correct_or_absent_limit_id() {
        let reset = 2_000_000_000i64;
        let correct = serde_json::json!({
            "rateLimits": [
                { "limitId": "codex", "usedPercent": 23.0, "windowDurationMins": 10080, "resetsAt": reset }
            ]
        });
        let (used, _) = parse_codex_rate_limits(&correct).unwrap();
        assert_eq!(used, Some(23.0));

        let absent = serde_json::json!({
            "rateLimits": [
                { "usedPercent": 40.0, "windowDurationMins": 10080, "resetsAt": reset }
            ]
        });
        let (used, _) = parse_codex_rate_limits(&absent).unwrap();
        assert_eq!(used, Some(40.0));

        // A wrong-limitId-only legacy list is not trusted: no weekly -> unknown.
        let wrong = serde_json::json!({
            "rateLimits": [
                { "limitId": "chatgpt", "usedPercent": 5.0, "windowDurationMins": 10080, "resetsAt": reset }
            ]
        });
        let (used, _) = parse_codex_rate_limits(&wrong).unwrap();
        assert_eq!(used, None);
    }

    #[test]
    fn codex_unknown_shape_fails() {
        assert_eq!(
            parse_codex_rate_limits(&serde_json::json!({"foo": 1}))
                .unwrap_err()
                .code,
            AppError::CODEX_BAD_RESPONSE
        );
    }

    // --- Hermes parse ------------------------------------------------------

    fn hermes_doc(used: &str, fetched: &str, reset: &str) -> String {
        format!(
            r#"{{"provider":"openai-codex","source":"usage_api","plan":"Prolite","fetched_at":"{fetched}","windows":[{{"label":"Weekly","used_percent":{used},"resets_at":"{reset}"}}],"unavailable_reason":null}}"#
        )
    }

    #[test]
    fn hermes_parses_weekly_window() {
        let body = hermes_doc(
            "23.0",
            "2026-09-29T19:05:02.086492+00:00",
            "2026-10-03T18:41:18+00:00",
        );
        let q = parse_hermes_quota(body.as_bytes()).unwrap();
        assert_eq!(q.source, "usage_api");
        assert_eq!(q.used_percent, Some(23.0));
        assert!(q.resets_at.is_some());
    }

    #[test]
    fn hermes_argv_is_exactly_the_read_only_usage_command() {
        assert_eq!(
            HERMES_ARGV,
            [
                "--profile",
                "default",
                "usage",
                "--provider",
                "openai-codex",
                "--json"
            ]
        );
    }

    #[test]
    fn hermes_rejects_wrong_provider_unavailable_and_bad_percent() {
        let wrong_provider = hermes_doc("23.0", "2026-09-29T19:05:02Z", "2026-10-03T18:41:18Z")
            .replace("openai-codex", "anthropic");
        assert!(parse_hermes_quota(wrong_provider.as_bytes()).is_err());

        let unavailable = hermes_doc("23.0", "2026-09-29T19:05:02Z", "2026-10-03T18:41:18Z")
            .replace(
                "\"unavailable_reason\":null",
                "\"unavailable_reason\":\"login required\"",
            );
        assert!(parse_hermes_quota(unavailable.as_bytes()).is_err());

        for bad in ["-1", "100.5"] {
            let body = hermes_doc(bad, "2026-09-29T19:05:02Z", "2026-10-03T18:41:18Z");
            assert!(
                parse_hermes_quota(body.as_bytes()).is_err(),
                "should reject {bad}"
            );
        }
    }

    #[test]
    fn hermes_missing_weekly_is_unknown_not_an_error() {
        let body = r#"{"provider":"openai-codex","source":"usage_api","fetched_at":"2026-09-29T19:05:02Z","windows":[],"unavailable_reason":null}"#;
        let q = parse_hermes_quota(body.as_bytes()).unwrap();
        assert_eq!(q.used_percent, None);
    }

    #[test]
    fn mood_helpers_are_reachable_from_models() {
        // Sanity cross-check that the mood math used by the service is wired.
        assert_eq!(crate::models::deepseek_mood(dec("42.50")), Mood::Happy);
        assert_eq!(crate::models::codex_mood(68.0), Mood::Happy);
    }

    // --- Executable discovery ----------------------------------------------

    #[test]
    fn codex_platform_pkg_maps_known_triples() {
        assert!(codex_platform_pkg().is_some());
        // Platform mapping is exercised indirectly below via the vendor search;
        // the mapping itself only returns Some on a known OS/ARCH combination.
    }

    #[test]
    fn cmd_and_bat_are_rejected_as_shell_shims() {
        assert!(is_cmd_or_bat(Path::new("C:\\npm\\codex.cmd")));
        assert!(is_cmd_or_bat(Path::new("C:\\npm\\codex.bat")));
        assert!(is_cmd_or_bat(Path::new("C:\\npm\\CODEX.CMD")));
        assert!(!is_cmd_or_bat(Path::new("C:\\npm\\codex.exe")));
        assert!(!is_cmd_or_bat(Path::new("/usr/local/bin/codex")));
    }

    #[test]
    fn discover_codex_finds_native_vendor_binary_in_node_modules() {
        // Only meaningful when the platform has an official npm package; the
        // vendor search tolerates any `vendor/<triple>` directory.
        let Some(pkg) = codex_platform_pkg() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let vendor_bin = dir
            .path()
            .join("node_modules")
            .join(format!("@openai/codex-{pkg}"))
            .join("vendor")
            .join("some-triple")
            .join("bin")
            .join(exe_name("codex"));
        std::fs::create_dir_all(vendor_bin.parent().unwrap()).unwrap();
        std::fs::write(&vendor_bin, b"").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&vendor_bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let found = find_codex_vendor_binary(&dir.path().join("node_modules"));
        assert!(found.is_some(), "vendor binary must be discovered");
        match found.unwrap() {
            Executable::Direct(p) => assert_eq!(p, vendor_bin),
        }
    }

    #[test]
    fn discover_codex_prefers_kvoteven_codex_env_override() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join(exe_name("codex"));
        std::fs::write(&target, b"").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // Absolute override path resolves. Setting process-global env in a test
        // must be serialized so parallel tests don't race on the variable.
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _g = LOCK.lock().unwrap();
        let prev = std::env::var("KVOTEVEN_CODEX").ok();
        std::env::set_var("KVOTEVEN_CODEX", &target);
        let found = discover_codex();
        match prev {
            Some(v) => std::env::set_var("KVOTEVEN_CODEX", v),
            None => std::env::remove_var("KVOTEVEN_CODEX"),
        }
        match found {
            Some(Executable::Direct(p)) => assert_eq!(p, target),
            other => panic!("expected Direct override, got {other:?}"),
        }
    }

    #[test]
    fn is_executable_rejects_nonexistent_and_non_files() {
        assert!(!is_executable(Path::new("/definitely/not/here")));
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_executable(&dir.path().join("subdir")));
    }
}
