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
//! and never expose it, so we recover the *single* thread of a just-created
//! suspended process through a bounded Toolhelp thread snapshot (a suspended
//! process has exactly one thread, so this is a fixed-size enumeration of that
//! process's threads, not a system-wide process scan).

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
/// `CREATE_SUSPENDED`. A just-created suspended process has exactly one thread,
/// so the Toolhelp snapshot below is a bounded, single-process enumeration.
fn resume_primary_thread(pid: u32) -> io::Result<()> {
    // SAFETY: all handles are closed exactly once; the snapshot entry is
    // re-initialised by `Thread32First`/`Thread32Next` per the Toolhelp
    // contract.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(io::Error::other)?;
        let mut entry = THREADENTRY32::default();
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
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
/// resume it. Returns the running child and the owning job (if assignment
/// succeeded). On assignment failure we degrade to direct-child-only cleanup
/// (`None` job) rather than failing the spawn: the child still runs and is
/// still killed directly, it just lacks descendant cleanup.
pub(crate) fn spawn_suspended(
    exec: &Executable,
    args: &[String],
) -> io::Result<(Child, Option<JobHandle>)> {
    let job = JobHandle::new()?;

    let mut cmd = exec.to_command();
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .creation_flags(CREATE_SUSPENDED.0);

    let mut child = cmd.spawn()?;

    let Some(raw) = child.raw_handle() else {
        // The child exited before we could assign it; nothing left to clean up.
        return Ok((child, None));
    };
    let Some(pid) = child.id() else {
        return Ok((child, None));
    };

    // Assign *before* the child executes so any descendant it spawns is born
    // into the job and dies with it.
    let assigned = job.assign(HANDLE(raw));

    // Always resume: a suspended child would otherwise leak forever. If the
    // resume itself fails, reap what we can and surface the error.
    if let Err(e) = resume_primary_thread(pid) {
        if assigned.is_ok() {
            job.terminate();
        }
        let _ = child.start_kill();
        return Err(e);
    }

    if assigned.is_err() {
        // Child is already in some other job (e.g. inherited from our own
        // process) and could not be moved. Direct-child kill still applies.
        return Ok((child, None));
    }

    Ok((child, Some(job)))
}
