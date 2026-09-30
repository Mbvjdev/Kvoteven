//! Cross-platform child-process runner with bounded output, a hard deadline
//! and guaranteed reap. It never invokes a shell: the program and arguments
//! are passed as a fixed array, so user-controlled strings are never
//! interpolated into a command line.
//!
//! On Unix the child is placed in its own process group and killing it kills
//! the whole group (best-effort: a descendant that calls `setsid`/daemonizes
//! is outside the group and is *not* guaranteed to be reaped). On Windows the
//! child is assigned to a kill-on-close Job Object *before it runs* (see the
//! `runner_windows` module), so a wrapper chain such as a pip launcher
//! (`hermes.exe` → `python.exe`) is reaped as a unit even on timeout, error or
//! cancellation — no wrapper chain is left orphaned.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};

#[cfg(windows)]
mod runner_windows;

/// A resolved native executable path. Never a shell script or `.cmd`/`.bat`
/// shim: those wrappers are rejected during discovery so we only ever spawn the
/// real binary and never interpolate arguments through a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Executable {
    Direct(PathBuf),
}

impl Executable {
    pub fn to_command(&self) -> Command {
        match self {
            Executable::Direct(p) => Command::new(p),
        }
    }
}

/// Errors surfaced by the runner; all details are local and safe to map to a
/// fixed `AppError` code upstream.
#[derive(Debug)]
pub enum RunError {
    Io(io::Error),
    TimedOut,
    OutputTooLarge,
    /// The process exited but with a non-zero status.
    ExitStatus(i32),
}

impl From<io::Error> for RunError {
    fn from(e: io::Error) -> Self {
        RunError::Io(e)
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Io(_) => f.write_str("process I/O error"),
            RunError::TimedOut => f.write_str("process timed out"),
            RunError::OutputTooLarge => f.write_str("process output too large"),
            RunError::ExitStatus(_) => f.write_str("process exited with an error"),
        }
    }
}

impl std::error::Error for RunError {}

/// Options for a bounded run.
#[derive(Debug, Clone)]
pub struct SpawnOptions {
    pub timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        SpawnOptions {
            timeout: Duration::from_secs(25),
            max_output_bytes: 1024 * 1024,
        }
    }
}

/// A spawned child with piped stdin/stdout and stderr sent to null.
pub struct SpawnedProcess {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: BufReader<tokio::process::ChildStdout>,
    /// On Windows: the kill-on-close Job Object owning this child and every
    /// process it spawns. Dropping or terminating it kills the whole tree.
    #[cfg(windows)]
    job: Option<runner_windows::JobHandle>,
}

/// Kill the direct child and, on Unix, its whole process group. `start_kill`
/// is always issued so the direct child is guaranteed to die even if the group
/// kill is a no-op (e.g. the child failed to become a group leader).
#[cfg(unix)]
fn kill_tree(child: &mut Child) {
    if let Some(pid) = child.id() {
        // `process_group(0)` made the child its own group leader, so its
        // pgid equals its pid and a negative-pid kill reaches descendants.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    let _ = child.start_kill();
}

/// Terminate the Job Object (killing the child and every descendant) and then
/// the direct child as a backstop for the degraded no-job path.
#[cfg(windows)]
fn kill_tree(child: &mut Child, job: &mut Option<runner_windows::JobHandle>) {
    if let Some(job) = job.take() {
        job.terminate();
    }
    let _ = child.start_kill();
}

impl SpawnedProcess {
    pub fn spawn(exec: &Executable, args: &[String]) -> io::Result<Self> {
        // Windows assigns the child to a kill-on-close job *before* resuming it
        // (see `runner_windows::spawn_suspended`); Unix puts it in its own
        // process group.
        #[cfg(windows)]
        let (mut child, job) = runner_windows::spawn_suspended(exec, args)?;

        #[cfg(unix)]
        let mut child = {
            let mut cmd = exec.to_command();
            cmd.args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .process_group(0);
            cmd.spawn()?
        };

        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "child stdout not piped"))?;
        Ok(SpawnedProcess {
            child: Some(child),
            stdin,
            stdout: BufReader::new(stdout),
            #[cfg(windows)]
            job,
        })
    }

    /// Send EOF to the child by dropping its stdin.
    pub fn close_stdin(&mut self) {
        self.stdin.take();
    }

    /// Write one line to the child's stdin.
    pub async fn write_line(&mut self, line: &str) -> io::Result<()> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdin already closed",
            ));
        };
        stdin.write_all(line.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await
    }

    /// Read one newline-delimited line, charging every byte consumed from the
    /// stream (content *and* delimiter) against `budget`. Returns `Ok(None)` on
    /// EOF and `Err(OutputTooLarge)` if the budget would be exceeded.
    ///
    /// Buffering is bounded *while* reading: input is consumed chunk-by-chunk
    /// via `fill_buf`/`consume` and the accumulated line never grows past the
    /// budget, so a process emitting bytes without any newline (or an endless
    /// run of blank lines) fails closed instead of exhausting memory.
    pub async fn read_line(&mut self, budget: &mut usize) -> Result<Option<String>, RunError> {
        let mut line: Vec<u8> = Vec::new();
        loop {
            let buf = self.stdout.fill_buf().await.map_err(RunError::Io)?;
            if buf.is_empty() {
                // EOF: any bytes already charged were accounted for above.
                return if line.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(String::from_utf8_lossy(&line).into_owned()))
                };
            }
            let newline = buf.iter().position(|&b| b == b'\n');
            let (consumed, content_len) = match newline {
                Some(pos) => (pos + 1, pos),
                None => (buf.len(), buf.len()),
            };
            if consumed > *budget {
                return Err(RunError::OutputTooLarge);
            }
            *budget -= consumed;
            line.extend_from_slice(&buf[..content_len]);
            self.stdout.consume(consumed);
            if newline.is_some() {
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }
        }
    }

    /// Terminate the child (and its process group on Unix / Job Object on
    /// Windows) if still running, then reap it. Always safe to call.
    pub async fn reap(&mut self) {
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => {}
                _ => {
                    #[cfg(unix)]
                    kill_tree(&mut child);
                    #[cfg(windows)]
                    kill_tree(&mut child, &mut self.job);
                }
            }
            let _ = child.wait().await;
        }
    }

    /// Wait for exit and return the status, without killing.
    pub async fn wait_status(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        match self.child.as_mut() {
            Some(child) => child.wait().await.map(Some),
            None => Ok(None),
        }
    }
}

impl Drop for SpawnedProcess {
    fn drop(&mut self) {
        // Guaranteed reap even on panic / early return / cancellation.
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            kill_tree(&mut child);
            #[cfg(windows)]
            kill_tree(&mut child, &mut self.job);
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let _ = child.wait().await;
                });
            }
            // Outside a runtime the kill signal has still been sent; the OS
            // reaps it.
        }
        // On Windows the Job Object (if any) is dropped here too, so
        // kill-on-close reaps any descendant that outlived the direct child.
    }
}

/// Run a program to completion, capturing bounded stdout. The child's stdin is
/// closed immediately; stderr goes to null. On timeout (or stdout EOF with the
/// process still running) the child is killed and reaped.
///
/// The deadline is shared by the read phase *and* the wait-for-exit phase: a
/// process that closes stdout and then keeps running is still bounded by
/// `opts.timeout` rather than hanging forever after EOF.
pub async fn run_bounded(
    exec: &Executable,
    args: &[String],
    opts: &SpawnOptions,
) -> Result<Vec<u8>, RunError> {
    let mut child = SpawnedProcess::spawn(exec, args)?;
    child.close_stdin();

    let result = tokio::time::timeout(opts.timeout, async {
        let mut budget = opts.max_output_bytes;
        let mut out = Vec::new();
        while let Some(line) = child.read_line(&mut budget).await? {
            out.extend_from_slice(line.as_bytes());
            out.push(b'\n');
        }
        // stdout EOF reached within budget; wait for process exit under the
        // same deadline.
        let status = child.wait_status().await?;
        if status.map(|s| s.success()).unwrap_or(false) {
            Ok(out)
        } else {
            let code = status.and_then(|s| s.code()).unwrap_or(-1);
            Err(RunError::ExitStatus(code))
        }
    })
    .await;

    match result {
        Err(_elapsed) => {
            child.reap().await;
            Err(RunError::TimedOut)
        }
        Ok(Err(e)) => {
            child.reap().await;
            Err(e)
        }
        Ok(Ok(out)) => Ok(out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn exe(name: &str) -> Executable {
        Executable::Direct(PathBuf::from(name))
    }
    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reads_stdout_only_from_a_real_process() {
        let out = run_bounded(
            &exe("/usr/bin/printf"),
            &args(&["verified-output"]),
            &SpawnOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out), "verified-output\n");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn nonzero_exit_is_never_valid_output() {
        let r = run_bounded(&exe("/usr/bin/false"), &args(&[]), &SpawnOptions::default()).await;
        assert!(matches!(r, Err(RunError::ExitStatus(_))));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stalled_read_is_terminated_within_deadline() {
        let start = std::time::Instant::now();
        let opts = SpawnOptions {
            timeout: Duration::from_millis(100),
            max_output_bytes: 1024,
        };
        let r = run_bounded(&exe("/bin/sleep"), &args(&["5"]), &opts).await;
        assert!(matches!(r, Err(RunError::TimedOut)));
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "child must be killed promptly"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn output_beyond_budget_fails_closed() {
        let opts = SpawnOptions {
            timeout: Duration::from_secs(5),
            max_output_bytes: 1024,
        };
        let r = run_bounded(&exe("/usr/bin/yes"), &args(&[]), &opts).await;
        assert!(matches!(r, Err(RunError::OutputTooLarge)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn no_newline_stream_is_memory_bounded() {
        // `dd` emits bytes without any newline; the read must fail closed on
        // the budget rather than buffering without bound.
        let opts = SpawnOptions {
            timeout: Duration::from_secs(5),
            max_output_bytes: 4096,
        };
        let r = run_bounded(&exe("/bin/dd"), &args(&["if=/dev/zero"]), &opts).await;
        assert!(matches!(r, Err(RunError::OutputTooLarge)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn endless_blank_lines_charge_the_delimiter() {
        // `yes ''` prints only newlines; each delimiter must count against the
        // budget so this fails closed instead of bloating the output buffer.
        let opts = SpawnOptions {
            timeout: Duration::from_secs(5),
            max_output_bytes: 1024,
        };
        let r = run_bounded(&exe("/usr/bin/yes"), &args(&[""]), &opts).await;
        assert!(matches!(r, Err(RunError::OutputTooLarge)));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_nonzero_exit_is_not_valid_output() {
        let r = run_bounded(
            &exe("cmd.exe"),
            &args(&["/C", "exit 7"]),
            &SpawnOptions::default(),
        )
        .await;
        assert!(matches!(r, Err(RunError::ExitStatus(7))));
    }
}
