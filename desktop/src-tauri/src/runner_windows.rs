//! Windows-only spawn path for the runner: a per-child Job Object that owns the
//! child and every process it spawns, so a pip-launcher (`hermes.exe` →
//! `python.exe`) or any other wrapper chain is reaped as a unit.
//!
//! On Windows a discovered "native" binary can still be a thin launcher that
//! immediately spawns the real interpreter and waits for it. Killing only the
//! launcher orphans the interpreter and its own children. The OS primitive for
//! "kill this process and everything it ever created" is a **Job Object** with
//! [`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`]: every process spawned by a job
//! member is automatically added to the same job, and closing the last handle
//! (or calling `TerminateJobObject`) kills the whole tree.
//!
//! The child must be assigned *before it runs any code*, otherwise a fast
//! launcher can spawn its interpreter in the gap between spawn and assign and
//! escape the job. We therefore spawn with `CREATE_SUSPENDED`, assign the
//! suspended process handle, and only then resume the primary thread. std /
//! tokio close the primary thread handle immediately after `CreateProcessW`
//! and never expose it, so we recover the thread of the just-created suspended
//! process from a Toolhelp thread snapshot.
//!
//! `TH32CS_SNAPTHREAD` always includes *every* thread in the system and its
//! `th32ProcessID` argument is ignored for this flag, so the snapshot is a
//! single, finite snapshot of the system-wide thread list — not a fixed-size,
//! per-process enumeration. We walk it once and keep only the entry whose
//! `th32OwnerProcessID` matches the suspended child; a just-created suspended
//! process has exactly one thread, so the first matching entry is its primary
//! thread. There is no recursive re-snapshotting or rescanning.

use std::ffi::c_void;
use std::io;
use std::process::Stdio;

use tokio::process::Child;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Threading::{
    OpenThread, ResumeThread, CREATE_SUSPENDED, THREAD_SUSPEND_RESUME,
};

use super::Executable;

/// An anonymous Windows Job Object configured with
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Dropping it closes the last handle and
/// the OS terminates every process still assigned to it — the direct child plus
/// any descendants it spawned while running.
pub(crate) struct JobHandle(HANDLE);

// A `HANDLE` is an opaque kernel index (`*mut c_void` that is never
// dereferenced); moving it across threads is sound. The same reasoning is used
// by the `win32job` crate for its job wrapper.
unsafe impl Send for JobHandle {}
unsafe impl Sync for JobHandle {}

impl JobHandle {
    fn new() -> io::Result<Self> {
        // SAFETY: `CreateJobObjectW`/`SetInformationJobObject` take pointers to
        // values that outlive the calls; the info struct is a plain value.
        unsafe {
            let handle = CreateJobObjectW(None, PCWSTR::null()).map_err(io::Error::other)?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if let Err(e) = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) {
                let _ = CloseHandle(handle);
                return Err(io::Error::other(e));
            }
            Ok(JobHandle(handle))
        }
    }

    fn assign(&self, process: HANDLE) -> io::Result<()> {
        // SAFETY: `process` is a valid, still-open handle to the suspended
        // child owned by the caller; `self.0` outlives the call.
        unsafe { AssignProcessToJobObject(self.0, process) }.map_err(io::Error::other)
    }

    /// Synchronously terminate every process in the job (the child and all
    /// descendants). Kill-on-close reaps anything left over when the handle is
    /// dropped.
    pub(crate) fn terminate(&self) {
        // SAFETY: `self.0` is a valid job handle for the lifetime of `self`.
        unsafe {
            let _ = TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        // Kill-on-close: closing the last handle terminates any process still
        // in the job (the child and any surviving descendants).
        // SAFETY: `self.0` is owned uniquely by this struct.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Resume the primary thread of `pid`, which must have been spawned with
/// `CREATE_SUSPENDED`. The thread is located from a single finite Toolhelp
/// thread snapshot: `TH32CS_SNAPTHREAD` always contains all system threads
/// (`th32ProcessID` is ignored for this flag), so entries are filtered by
/// owner PID. A just-created suspended process has exactly one thread, so the
/// first matching entry is its primary thread.
fn resume_primary_thread(pid: u32) -> io::Result<()> {
    // SAFETY: all handles are closed exactly once; the snapshot entry is
    // re-initialised by `Thread32First`/`Thread32Next` per the Toolhelp
    // contract.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(io::Error::other)?;
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        let mut thread_id = None;
        if Thread32First(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32OwnerProcessID == pid {
                    thread_id = Some(entry.th32ThreadID);
                    break;
                }
                if Thread32Next(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);

        let thread_id =
            thread_id.ok_or_else(|| io::Error::other("suspended child has no primary thread"))?;
        let thread =
            OpenThread(THREAD_SUSPEND_RESUME, false, thread_id).map_err(io::Error::other)?;
        // `ResumeThread` returns the previous suspend count, or `u32::MAX` on
        // failure (it is not a `Result`).
        let previous = ResumeThread(thread);
        let _ = CloseHandle(thread);
        if previous == u32::MAX {
            return Err(io::Error::other("ResumeThread failed"));
        }
        Ok(())
    }
}

/// Spawn `exec args` suspended, assign it to a fresh kill-on-close job, then
/// resume it. Returns the running child and its owning job.
///
/// Fails closed: if the child cannot be assigned to the job (or its primary
/// thread cannot be resumed), the still-suspended child is killed and a
/// sanitized I/O error is returned — the helper is never allowed to run
/// outside the job.
pub(crate) fn spawn_suspended(
    exec: &Executable,
    args: &[String],
) -> io::Result<(Child, Option<JobHandle>)> {
    spawn_suspended_with_assign(exec, args, |job, process| job.assign(process))
}

/// The assignment step is injected so a regression test can simulate a failed
/// assignment; production always uses `JobHandle::assign`.
fn spawn_suspended_with_assign<F>(
    exec: &Executable,
    args: &[String],
    assign: F,
) -> io::Result<(Child, Option<JobHandle>)>
where
    F: FnOnce(&JobHandle, HANDLE) -> io::Result<()>,
{
    let job = JobHandle::new()?;

    let mut cmd = exec.to_command();
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .creation_flags(CREATE_SUSPENDED.0);

    let mut child = cmd.spawn()?;

    // Assign *before* the child executes so any descendant it spawns is born
    // into the job and dies with it. Any failure before assign + resume both
    // succeed must fail closed: the child is still suspended, so kill it (never
    // resume it) and surface a sanitized error rather than letting it run
    // outside the job.
    let outcome: io::Result<()> = (|| {
        let raw = child
            .raw_handle()
            .ok_or_else(|| io::Error::other("spawned child has no raw process handle"))?;
        let pid = child
            .id()
            .ok_or_else(|| io::Error::other("spawned child has no process id"))?;
        assign(&job, HANDLE(raw))?;
        resume_primary_thread(pid)
    })();

    if let Err(e) = outcome {
        // Fail closed. `terminate` reaps the job if assignment succeeded but
        // resume failed, and is a no-op if assignment itself failed; the direct
        // child is then killed (and reaped on drop via `kill_on_drop`) so it can
        // never execute the helper.
        job.terminate();
        let _ = child.start_kill();
        return Err(e);
    }

    Ok((child, Some(job)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Regression: an assignment failure must fail closed — the helper must
    /// never run — instead of resuming the child and returning `Ok` with no job
    /// (which would let it execute outside any kill-on-close job).
    #[tokio::test]
    async fn assignment_failure_fails_closed_and_never_runs_helper() {
        let marker = std::env::temp_dir().join(format!(
            "kvoteven_assignment_failure_{}.tmp",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&marker);

        let exec = Executable::Direct(PathBuf::from("cmd.exe"));
        let args = vec![
            "/C".to_string(),
            format!("type nul > \"{}\"", marker.display()),
        ];

        let result = spawn_suspended_with_assign(&exec, &args, |_job, _process| {
            Err(io::Error::other("simulated assignment failure"))
        });

        match result {
            Ok(_) => panic!("assignment failure must fail closed, not return Ok"),
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::Other),
        }

        assert!(
            !marker.exists(),
            "helper executed despite failed assignment; marker file was created"
        );
        let _ = std::fs::remove_file(&marker);
    }
}
