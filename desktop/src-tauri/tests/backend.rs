//! Tracked, native cross-platform integration tests for the runner and the
//! Codex app-server protocol. They drive a real spawned child (a dependency-free
//! probe compiled on demand with `rustc`) so cleanup, bounded output, deadlines
//! and the exact JSON-RPC handshake are all exercised against a live process.
//!
//! These run on every `cargo test` on every platform (no `#[ignore]`); the
//! Windows-specific assertions (e.g. `.cmd`/`.bat` rejection) are compiled for
//! Windows and exercised when CI runs on Windows.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use kvoteven_lib::models::APP_VERSION;
use kvoteven_lib::providers::{
    codex_protocol, parse_codex_rate_limits, read_codex_quota, read_hermes_quota,
};
#[cfg(windows)]
use kvoteven_lib::runner::SpawnedProcess;
use kvoteven_lib::runner::{run_bounded, Executable, RunError, SpawnOptions};

/// Compile `tests/support/probe.rs` once with `rustc` and reuse the binary.
/// Avoids a Cargo `[[bin]]` target, which would break the Tauri bundler's
/// automatic binary selection.
fn probe() -> Executable {
    static PROBE: OnceLock<PathBuf> = OnceLock::new();
    Executable::Direct(PROBE.get_or_init(compile_probe).clone())
}

fn compile_probe() -> PathBuf {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/probe.rs");
    let out_dir = std::env::temp_dir().join(format!("kvoteven-probe-{}", std::process::id()));
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join(format!("probe{}", std::env::consts::EXE_SUFFIX));
    let status = Command::new("rustc")
        .arg(&src)
        .arg("-o")
        .arg(&exe)
        .arg("--edition=2021")
        .status()
        .expect("rustc must be available to compile the test probe");
    assert!(status.success(), "probe compilation failed");
    exe
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// --- runner -----------------------------------------------------------------

#[tokio::test]
async fn runner_echo_and_nonzero_exit() {
    let out = run_bounded(
        &probe(),
        &args(&["echo", "hello"]),
        &SpawnOptions::default(),
    )
    .await
    .unwrap();
    assert_eq!(String::from_utf8_lossy(&out), "hello\n");

    let r = run_bounded(&probe(), &args(&["fail"]), &SpawnOptions::default()).await;
    assert!(matches!(r, Err(RunError::ExitStatus(3))));
}

#[tokio::test]
async fn runner_timeout_kills_stalled_probe() {
    let start = std::time::Instant::now();
    let opts = SpawnOptions {
        timeout: Duration::from_millis(150),
        max_output_bytes: 1024 * 1024,
    };
    let r = run_bounded(&probe(), &args(&["sleep", "10"]), &opts).await;
    assert!(matches!(r, Err(RunError::TimedOut)));
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "probe must be killed promptly"
    );
}

#[tokio::test]
async fn runner_budget_fails_closed() {
    let opts = SpawnOptions {
        timeout: Duration::from_secs(10),
        max_output_bytes: 1024,
    };
    let r = run_bounded(&probe(), &args(&["big"]), &opts).await;
    assert!(matches!(r, Err(RunError::OutputTooLarge)));
}

#[tokio::test]
async fn runner_no_newline_stream_is_memory_bounded() {
    let opts = SpawnOptions {
        timeout: Duration::from_secs(10),
        max_output_bytes: 4096,
    };
    let r = run_bounded(&probe(), &args(&["nonewline"]), &opts).await;
    assert!(matches!(r, Err(RunError::OutputTooLarge)));
}

#[tokio::test]
async fn runner_endless_blank_lines_charge_the_delimiter() {
    let opts = SpawnOptions {
        timeout: Duration::from_secs(10),
        max_output_bytes: 1024,
    };
    let r = run_bounded(&probe(), &args(&["blanklines"]), &opts).await;
    assert!(matches!(r, Err(RunError::OutputTooLarge)));
}

#[tokio::test]
async fn runner_close_stdout_then_sleep_is_bounded_by_the_deadline() {
    // The probe closes stdout (reader sees EOF) but keeps running. The runner
    // must apply its deadline to the wait-for-exit phase too, not just reads.
    let start = std::time::Instant::now();
    let opts = SpawnOptions {
        timeout: Duration::from_millis(200),
        max_output_bytes: 1024 * 1024,
    };
    let r = run_bounded(&probe(), &args(&["close-then-sleep"]), &opts).await;
    assert!(matches!(r, Err(RunError::TimedOut)));
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "wait phase must share the read deadline"
    );
}

/// True when a pid is still alive (zombies count until reaped). Uses the POSIX
/// `kill -0` probe so the test stays dependency-free.
#[cfg(unix)]
fn pid_alive(pid: i32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(unix)]
#[tokio::test]
async fn runner_process_group_kill_reaches_descendants() {
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("grandchild.pid");
    let opts = SpawnOptions {
        timeout: Duration::from_secs(2),
        max_output_bytes: 1024 * 1024,
    };
    let r = run_bounded(&probe(), &args(&["tree", pidfile.to_str().unwrap()]), &opts).await;
    assert!(matches!(r, Err(RunError::TimedOut)));

    // The grandchild wrote its pid before the timeout; it must be reaped with
    // the process group. Poll for the zombie to be collected.
    let pid: i32 = std::fs::read_to_string(&pidfile)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let mut dead = false;
    for _ in 0..40 {
        if !pid_alive(pid) {
            dead = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        dead,
        "grandchild {pid} must be reaped with the process group"
    );
}

/// True when a Windows pid is still alive. Uses `OpenProcess` +
/// `GetExitCodeProcess` (`STILL_ACTIVE`) so the test stays dependency-free.
#[cfg(windows)]
fn pid_alive(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => {
                let mut code = 0u32;
                let alive =
                    GetExitCodeProcess(handle, &mut code).is_ok() && code == STILL_ACTIVE.0 as u32;
                let _ = CloseHandle(handle);
                alive
            }
            // ERROR_INVALID_PARAMETER: no such process.
            Err(_) => false,
        }
    }
}

#[cfg(windows)]
fn wait_for_pidfile(path: &std::path::Path) {
    for _ in 0..40 {
        if path.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("grandchild never wrote its pidfile");
}

/// The probe's `tree` mode spawns a grandchild as its very first action — the
/// same "launcher spawns interpreter immediately" shape that would otherwise
/// race the job assignment. The grandchild must die with the Job Object even
/// though only the direct child was killed by name.
#[cfg(windows)]
#[tokio::test]
async fn windows_job_object_kills_descendants_on_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("grandchild.pid");
    let opts = SpawnOptions {
        timeout: Duration::from_secs(2),
        max_output_bytes: 1024 * 1024,
    };
    let r = run_bounded(&probe(), &args(&["tree", pidfile.to_str().unwrap()]), &opts).await;
    assert!(matches!(r, Err(RunError::TimedOut)));

    let pid: u32 = std::fs::read_to_string(&pidfile)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let mut dead = false;
    for _ in 0..40 {
        if !pid_alive(pid) {
            dead = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(dead, "grandchild {pid} must die with the Job Object");
}

/// Dropping the `SpawnedProcess` (success/error/cancellation path) must close
/// the kill-on-close Job Object and reap the grandchild, not just the direct
/// child.
#[cfg(windows)]
#[tokio::test]
async fn windows_job_object_kills_descendants_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("grandchild.pid");
    let child =
        SpawnedProcess::spawn(&probe(), &args(&["tree", pidfile.to_str().unwrap()])).unwrap();
    wait_for_pidfile(&pidfile);
    let pid: u32 = std::fs::read_to_string(&pidfile)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    drop(child);

    let mut dead = false;
    for _ in 0..40 {
        if !pid_alive(pid) {
            dead = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        dead,
        "grandchild {pid} must die when the SpawnedProcess is dropped"
    );
}

// --- codex app-server protocol ----------------------------------------------

fn codex_log_contains(log: &std::path::Path, needle: &str) -> bool {
    std::fs::read_to_string(log)
        .map(|s| s.lines().any(|l| l == needle))
        .unwrap_or(false)
}

#[tokio::test]
async fn codex_exact_handshake_and_parse() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("handshake.log");
    let a = vec![
        "app-server".to_string(),
        "ok".to_string(),
        "--version".to_string(),
        APP_VERSION.to_string(),
        "--log".to_string(),
        log.to_str().unwrap().to_string(),
    ];
    let result = codex_protocol(&probe(), &a, Duration::from_secs(10))
        .await
        .expect("protocol succeeds");
    let (used, resets) = parse_codex_rate_limits(&result).expect("parse succeeds");
    assert_eq!(used, Some(32.0));
    assert!(resets.is_some());

    // The mock server asserted the exact handshake step-by-step; require every
    // step to have passed (not merely "responded by id").
    assert!(
        codex_log_contains(&log, "init_ok=true"),
        "initialize must carry only clientInfo"
    );
    assert!(
        codex_log_contains(&log, "initialized_ok=true"),
        "initialized must be exact"
    );
    assert!(
        codex_log_contains(&log, "ratelimits_ok=true"),
        "account/rateLimits/read must be exact"
    );
}

#[tokio::test]
async fn codex_init_error_aborts_before_further_messages() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("handshake.log");
    let a = vec![
        "app-server".to_string(),
        "init-error".to_string(),
        "--version".to_string(),
        APP_VERSION.to_string(),
        "--log".to_string(),
        log.to_str().unwrap().to_string(),
    ];
    let result = codex_protocol(&probe(), &a, Duration::from_secs(10)).await;
    assert!(matches!(result, Err(RunError::Io(_))));
    assert!(codex_log_contains(&log, "init_error_prepared=true"));
}

#[tokio::test]
async fn codex_rate_limits_error_is_sanitized_io_error() {
    let result = codex_protocol(
        &probe(),
        &args(&["app-server", "error"]),
        Duration::from_secs(10),
    )
    .await;
    assert!(matches!(result, Err(RunError::Io(_))));
}

#[tokio::test]
async fn codex_server_request_collision_is_rejected() {
    let result = codex_protocol(
        &probe(),
        &args(&["app-server", "server-request"]),
        Duration::from_secs(10),
    )
    .await;
    assert!(matches!(result, Err(RunError::Io(_))));
}

#[tokio::test]
async fn codex_ignores_notifications_and_finds_leaf() {
    let result = codex_protocol(
        &probe(),
        &args(&["app-server", "notify"]),
        Duration::from_secs(10),
    )
    .await
    .expect("protocol succeeds");
    let (used, _) = parse_codex_rate_limits(&result).unwrap();
    assert_eq!(used, Some(10.0));
}

#[tokio::test]
async fn codex_missing_weekly_is_none_not_error() {
    let result = codex_protocol(
        &probe(),
        &args(&["app-server", "no-weekly"]),
        Duration::from_secs(10),
    )
    .await
    .expect("protocol succeeds");
    let (used, _) = parse_codex_rate_limits(&result).unwrap();
    assert_eq!(used, None);
}

#[tokio::test]
async fn codex_stall_times_out_and_reaps() {
    let start = std::time::Instant::now();
    let result = codex_protocol(
        &probe(),
        &args(&["app-server", "stall"]),
        Duration::from_millis(500),
    )
    .await;
    assert!(matches!(result, Err(RunError::TimedOut)));
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn read_codex_quota_end_to_end() {
    let quota = read_codex_quota(&probe())
        .await
        .expect("read_codex_quota succeeds");
    assert_eq!(quota.source, "codex_app_server");
    assert_eq!(quota.used_percent, Some(32.0));
    assert!(quota.resets_at.is_some());
}

#[tokio::test]
async fn read_hermes_quota_end_to_end() {
    let quota = read_hermes_quota(&probe())
        .await
        .expect("hermes read succeeds");
    assert_eq!(quota.source, "usage_api");
    assert_eq!(quota.used_percent, Some(23.0));
    assert!(quota.resets_at.is_some());
}

#[tokio::test]
async fn codex_process_failure_maps_to_fixed_code() {
    let bad = Executable::Direct(PathBuf::from("/definitely/not/here"));
    let err = read_codex_quota(&bad).await.unwrap_err();
    assert_eq!(err.code, "codex_request_failed");
}

#[tokio::test]
async fn hermes_process_failure_maps_to_fixed_code() {
    let bad = Executable::Direct(PathBuf::from("/definitely/not/here"));
    let err = read_hermes_quota(&bad).await.unwrap_err();
    assert_eq!(err.code, "hermes_request_failed");
}
