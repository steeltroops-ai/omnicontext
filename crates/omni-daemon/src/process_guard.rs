//! Singleton Process Guard for the `OmniContext` daemon.
//!
//! Prevents multiple daemon instances from competing over the same SQLite
//! database for a given repository path.
//!
//! # Safety
//!
//! This module requires unsafe blocks for platform-specific process management
//! (`OpenProcess`, `CloseHandle`, `kill`) which have no safe Rust equivalents.
//! All usages are audited for correct handle lifetimes and argument validity.
#![allow(unsafe_code)]
//! ## Mechanism
//!
//! A **PID file** is written to the data directory on startup.  If a PID file
//! already exists when a new instance starts, the guard:
//!
//! 1. Reads the existing PID.
//! 2. Checks whether that process is still running.
//! 3. If running:
//!    a. Sends a `SIGTERM` (Unix) / `TerminateProcess` (Windows) graceful signal.
//!    b. Waits up to `GRACEFUL_SHUTDOWN_MS` for the process to exit.
//!    c. If still alive after the grace period, force-kills it.
//! 4. Deletes the stale PID file.
//! 5. Writes the current PID and continues startup.
//!
//! On clean shutdown the guard removes its own PID file so the next instance
//! starts without a conflict check.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;

/// Milliseconds to wait for the old process to exit gracefully before killing it.
const GRACEFUL_SHUTDOWN_MS: u64 = 3_000;
/// Poll interval while waiting for the old process to exit.
const POLL_INTERVAL_MS: u64 = 50;

/// Returned by `ProcessGuard::acquire`. Drop to release the PID file.
pub struct ProcessGuard {
    pid_file: PathBuf,
}

impl ProcessGuard {
    /// Acquire the singleton guard for the given data directory.
    ///
    /// Blocks (synchronously) until:
    /// - the previous instance has exited, or
    /// - `GRACEFUL_SHUTDOWN_MS` has elapsed and the old process is force-killed.
    ///
    /// Returns an error only if the PID file cannot be written.
    pub fn acquire(data_dir: &Path) -> Result<Self> {
        let pid_file = data_dir.join("daemon.pid");
        let current_pid = std::process::id();

        if pid_file.exists() {
            if let Ok(contents) = std::fs::read_to_string(&pid_file) {
                if let Ok(old_pid) = contents.trim().parse::<u32>() {
                    if old_pid != current_pid && is_process_running(old_pid) {
                        tracing::warn!(
                            pid = old_pid,
                            "stale daemon detected; requesting graceful shutdown"
                        );
                        request_shutdown(old_pid);

                        // Wait for graceful exit
                        let deadline = Instant::now() + Duration::from_millis(GRACEFUL_SHUTDOWN_MS);
                        while Instant::now() < deadline && is_process_running(old_pid) {
                            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
                        }

                        if is_process_running(old_pid) {
                            tracing::warn!(
                                pid = old_pid,
                                "graceful shutdown timed out; force-killing"
                            );
                            force_kill(old_pid);
                            // Give the OS a brief moment to release file handles / locks
                            std::thread::sleep(Duration::from_millis(200));
                        } else {
                            tracing::info!(pid = old_pid, "previous daemon exited cleanly");
                        }
                    }
                }
            }
            // Remove stale PID file regardless
            let _ = std::fs::remove_file(&pid_file);
        }

        // Write our own PID
        std::fs::create_dir_all(data_dir)?;
        std::fs::write(&pid_file, current_pid.to_string())?;
        tracing::debug!(pid = current_pid, path = %pid_file.display(), "process guard acquired");

        Ok(Self { pid_file })
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.pid_file);
        tracing::debug!(path = %self.pid_file.display(), "process guard released");
    }
}

// ---------------------------------------------------------------------------
// Platform helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the process with `pid` is currently running.
#[allow(unsafe_code)]
fn is_process_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        // SAFETY: OpenProcess is an OS API call; handle validity is checked immediately.
        let handle = unsafe {
            windows_sys::Win32::System::Threading::OpenProcess(
                // PROCESS_QUERY_LIMITED_INFORMATION (0x1000) — minimal rights
                0x1000, 0, // bInheritHandle = FALSE
                pid,
            )
        };
        if handle.is_null() {
            return false; // process doesn't exist or we can't access it
        }
        let mut exit_code: u32 = 0;
        // SAFETY: handle is a valid, non-null process handle from OpenProcess.
        let still_active = unsafe {
            windows_sys::Win32::System::Threading::GetExitCodeProcess(handle, &mut exit_code) != 0
                && exit_code == 259 // STILL_ACTIVE
        };
        // SAFETY: handle was opened by us and must be closed to avoid resource leak.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
        still_active
    }

    #[cfg(unix)]
    {
        // Sending signal 0 to a PID checks existence without disturbing the process.
        // SAFETY: kill(pid, 0) is always safe — it never delivers a real signal.
        let pid_t = pid as libc::pid_t;
        unsafe { libc::kill(pid_t, 0) == 0 }
    }

    #[cfg(not(any(windows, unix)))]
    {
        // Unsupported platform — assume process is not running
        let _ = pid;
        false
    }
}

/// Ask the process to terminate gracefully.
#[allow(unsafe_code)]
fn request_shutdown(pid: u32) {
    #[cfg(windows)]
    {
        // On Windows send WM_CLOSE is tricky for non-GUI processes; use CTRL_C_EVENT
        // to the process's console group, which is the closest equivalent to SIGTERM.
        // If that fails, fall through to force_kill after the grace period.
        let _ = pid; // suppress unused warning if ctrlc fails silently
        unsafe {
            // Attach to the target process's console group and send CTRL_C
            if windows_sys::Win32::System::Console::AttachConsole(pid) != 0 {
                windows_sys::Win32::System::Console::GenerateConsoleCtrlEvent(0, pid);
                windows_sys::Win32::System::Console::FreeConsole();
            }
        }
    }

    #[cfg(unix)]
    {
        // SAFETY: SIGTERM is a standard termination signal.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    }

    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
    }
}

/// Forcibly terminate the process.
#[allow(unsafe_code)]
fn force_kill(pid: u32) {
    #[cfg(windows)]
    {
        // SAFETY: OpenProcess with PROCESS_TERMINATE to get handle for termination.
        let handle = unsafe {
            windows_sys::Win32::System::Threading::OpenProcess(
                0x0001, // PROCESS_TERMINATE
                0, pid,
            )
        };
        if !handle.is_null() {
            // SAFETY: handle is valid; terminate the process and close the handle.
            unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(handle, 1);
                windows_sys::Win32::Foundation::CloseHandle(handle);
            }
        }
    }

    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    }

    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_creates_pid_file() {
        let dir = tempfile::tempdir().unwrap();
        let guard = ProcessGuard::acquire(dir.path()).unwrap();

        let pid_file = dir.path().join("daemon.pid");
        assert!(pid_file.exists(), "PID file should be created");

        let written_pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(written_pid, std::process::id());

        drop(guard);
        assert!(!pid_file.exists(), "PID file should be removed on drop");
    }

    #[test]
    fn test_guard_cleans_stale_nonexistent_pid() {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("daemon.pid");

        // Write a PID that almost certainly does not exist
        std::fs::write(&pid_file, "999999999").unwrap();

        // Should succeed — stale PID file with no running process
        let guard = ProcessGuard::acquire(dir.path());
        assert!(guard.is_ok());
    }

    #[test]
    fn test_is_own_process_running() {
        let own_pid = std::process::id();
        assert!(is_process_running(own_pid), "our own PID should be running");
    }

    #[test]
    fn test_nonexistent_pid_not_running() {
        // PID 999999999 is extremely unlikely to exist
        assert!(!is_process_running(999_999_999));
    }
}
