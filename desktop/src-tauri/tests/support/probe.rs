//! Cross-platform probe process for the tracked integration tests under
//! `desktop/src-tauri/tests`. It is compiled on demand with `rustc` (no extra
//! Cargo `[[bin]]`, which would break the Tauri bundler's binary auto-
//! selection) and drives two behaviours:
//!
//! * runner scenarios (`echo`, `sleep`, `fail`, `big`, `nonewline`,
//!   `blanklines`, `close-then-sleep`, `tree`, `record-then-sleep`) — exercise
//!   bounded output, deadlines and process-group kill.
//! * `app-server` — a strict Codex app-server mock that *validates the exact
//!   JSON-RPC handshake* (initialize with only `clientInfo`, then `initialized`,
//!   then `account/rateLimits/read`) and fails the exchange if the client
//!   deviates. It is pure `std`: byte-level assertions, no JSON parser.

use std::io::{BufRead, Write};
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Pull `--log <path>` out of argv so tests can inspect per-step assertions.
    let mut log: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--log" {
            log = args.get(i + 1).cloned();
            i += 2;
        } else {
            rest.push(args[i].clone());
            i += 1;
        }
    }
    match rest.first().map(String::as_str) {
        Some("echo") => {
            if let Some(t) = rest.get(1) {
                println!("{t}");
            }
        }
        Some("sleep") => {
            let secs: f64 = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            thread::sleep(Duration::from_secs_f64(secs));
        }
        Some("fail") => std::process::exit(3),
        Some("big") => {
            let line = "x".repeat(64);
            for _ in 0..200_000 {
                println!("{line}");
            }
        }
        Some("nonewline") => {
            // Unbounded bytes with no newline: the reader must fail closed on
            // its budget instead of buffering forever.
            let buf = [b'x'; 4096];
            let mut out = std::io::stdout();
            loop {
                if out.write_all(&buf).is_err() {
                    return;
                }
            }
        }
        Some("blanklines") => {
            // Many blank lines: each delimiter must count against the budget.
            let mut out = std::io::stdout();
            for _ in 0..100_000 {
                if out.write_all(b"\n").is_err() {
                    return;
                }
            }
        }
        Some("close-then-sleep") => {
            // Close stdout (reader sees EOF) but keep the process alive: the
            // runner's wait-for-exit phase must share the read deadline.
            close_stdout();
            thread::sleep(Duration::from_secs(30));
        }
        Some("tree") => {
            // Spawn a grandchild (same process group) then sleep; used to prove
            // process-group kill reaches descendants on Unix.
            let pidfile = rest.get(1).cloned().unwrap_or_default();
            let self_exe = std::env::current_exe().unwrap();
            let _ = std::process::Command::new(self_exe)
                .arg("record-then-sleep")
                .arg(&pidfile)
                .spawn();
            thread::sleep(Duration::from_secs(60));
        }
        Some("record-then-sleep") => {
            let pidfile = rest.get(1).cloned().unwrap_or_default();
            let _ = std::fs::write(&pidfile, std::process::id().to_string());
            thread::sleep(Duration::from_secs(60));
        }
        Some("app-server") => {
            let scenario = rest.get(1).map(String::as_str).unwrap_or("ok");
            let expected_version = rest
                .iter()
                .position(|a| a == "--version")
                .and_then(|p| rest.get(p + 1))
                .cloned()
                .unwrap_or_else(|| "0.4.0".to_string());
            codex_app_server(scenario, &expected_version, log.as_deref());
        }
        Some("--profile") => hermes_usage(&rest),
        _ => {}
    }
}

/// Close the process's own stdout so the reader observes EOF. Raw FFI keeps the
/// probe dependency-free (it is compiled with plain `rustc`).
#[cfg(unix)]
fn close_stdout() {
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    unsafe {
        close(1);
    }
}

#[cfg(windows)]
fn close_stdout() {
    extern "system" {
        fn GetStdHandle(nStdHandle: u32) -> isize;
        fn CloseHandle(hObject: isize) -> i32;
    }
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5u32; // (DWORD)-11
    unsafe {
        let h = GetStdHandle(STD_OUTPUT_HANDLE);
        if h != -1 && h != 0 {
            CloseHandle(h);
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn close_stdout() {}

fn log_line(path: Option<&str>, line: &str) {
    if let Some(p) = path {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

fn read_line_stdin() -> Option<String> {
    let mut s = String::new();
    let stdin = std::io::stdin();
    match stdin.lock().read_line(&mut s) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(s),
    }
}

/// Strict app-server mock. Validates the *exact* wire handshake byte-shape and
/// only then serves the requested scenario. Any deviation is logged and the
/// exchange is failed.
fn codex_app_server(scenario: &str, expected_version: &str, log: Option<&str>) {
    let mut out = std::io::stdout();
    let reset = now_unix() + 6 * 86400;

    // 1. initialize — must carry only `clientInfo`, no protocolVersion, no
    //    capabilities.
    let init = match read_line_stdin() {
        Some(l) => l,
        None => return,
    };
    let init_ok = !init.contains("protocolVersion")
        && !init.contains("capabilities")
        && init.contains("\"method\":\"initialize\"")
        && init.contains("\"id\":1")
        && init.contains("\"name\":\"kvoteven\"")
        && init.contains("\"title\":\"Kvoteven\"")
        && init.contains(&format!("\"version\":\"{expected_version}\""));
    log_line(log, &format!("init_ok={init_ok}"));

    if !init_ok || scenario == "init-error" {
        // Fail the handshake: the client must see an error and abort before
        // sending `initialized` / `account/rateLimits/read`.
        let _ = writeln!(
            out,
            "{}",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"denied"}}"#
        );
        let _ = out.flush();
        log_line(log, "init_error_sent=true");
        // The client must not send anything further; exit so any further write
        // would fail.
        return;
    }

    // Respond to initialize with a result.
    let _ = writeln!(
        out,
        "{}",
        r#"{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"codex","version":"0.159.2"}}}"#
    );
    let _ = out.flush();

    // 2. initialized notification (exactly `initialized`, not MCP's
    //    `notifications/initialized`, and no id).
    let init_notif = match read_line_stdin() {
        Some(l) => l,
        None => return,
    };
    let notif_ok = init_notif.contains("\"method\":\"initialized\"")
        && !init_notif.contains("notifications/initialized")
        && !init_notif.contains("\"id\"");
    log_line(log, &format!("initialized_ok={notif_ok}"));

    // 3. account/rateLimits/read
    let read = match read_line_stdin() {
        Some(l) => l,
        None => return,
    };
    let read_ok =
        read.contains("\"method\":\"account/rateLimits/read\"") && read.contains("\"id\":2");
    log_line(log, &format!("ratelimits_ok={read_ok}"));

    match scenario {
        "notify" => {
            for _ in 0..5 {
                let _ = writeln!(
                    out,
                    "{}",
                    r#"{"jsonrpc":"2.0","method":"notifications/progress","params":{}}"#
                );
            }
            let _ = writeln!(out, "{}", rate_limits_response(2, 10, 10080, reset));
        }
        "no-weekly" => {
            let _ = writeln!(out, "{}", rate_limits_response(2, 10, 4320, reset));
        }
        "error" => {
            let _ = writeln!(
                out,
                "{}",
                r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32000,"message":"denied"}}"#
            );
        }
        "server-request" => {
            // A JSON-RPC *request* from the server colliding with our id: the
            // client must reject it.
            let _ = writeln!(
                out,
                "{}",
                r#"{"jsonrpc":"2.0","id":2,"method":"some/request","params":{}}"#
            );
        }
        "stall" => {
            thread::sleep(Duration::from_secs(120));
        }
        _ => {
            // "ok" and anything else: respond with a weekly codex window.
            let _ = writeln!(out, "{}", rate_limits_response(2, 32, 10080, reset));
        }
    }
    let _ = out.flush();
}

fn rate_limits_response(id: u64, used: u64, window_mins: u64, reset: i64) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"result":{{"rateLimitsByLimitId":{{"codex":{{"primary":{{"usedPercent":{used},"windowDurationMins":{window_mins},"resetsAt":{reset}}},"secondary":{{"usedPercent":0,"windowDurationMins":{window_mins},"resetsAt":{reset}}}}}}}}}}}"#
    )
}

/// Emulate `hermes --profile default usage --provider openai-codex --json`.
fn hermes_usage(args: &[String]) {
    if args.iter().any(|a| a == "--bad") {
        println!(
            r#"{{"provider":"anthropic","source":"usage_api","fetched_at":"2026-09-29T19:05:02Z","windows":[],"unavailable_reason":null}}"#
        );
        return;
    }
    println!(
        r#"{{"provider":"openai-codex","source":"usage_api","plan":"Prolite","fetched_at":"2026-09-29T19:05:02Z","windows":[{{"label":"Weekly","used_percent":23.0,"resets_at":"2026-10-03T18:41:18+00:00"}}],"unavailable_reason":null}}"#
    );
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
