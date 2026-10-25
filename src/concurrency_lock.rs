//! Cross-process advisory concurrency lock for simulation workers.
//!
//! Complies with `AGENTS.md` concurrency policy:
//! "Because the CLI has no cross-process quota coordination and the per-dispatch
//! sleep is politeness rather than a cross-process rate limiter, do not run multiple
//! concurrent CLI screening processes at the same time or increase concurrency above 3
//! until cross-process rate limiting and platform-quota handling are in place."
//!
//! This module provides an advisory cross-process lock on `~/.cache/alphafind/sim_runner.lock`
//! (or a user-specified path) via POSIX `flock(2)`, ensuring mutual exclusion across all
//! CLI processes and helper scripts.

use anyhow::{anyhow, Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;

/// Default name of the cross-process lock file.
pub const DEFAULT_LOCK_FILE_NAME: &str = "sim_runner.lock";

/// Returns the default path to the simulation runner lock file (`~/.cache/alphafind/sim_runner.lock`).
pub fn get_default_lock_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".cache")
        .join("alphafind")
        .join(DEFAULT_LOCK_FILE_NAME)
}

/// An active advisory cross-process lock.
///
/// Automatically unlocks and closes the file descriptor when dropped (RAII).
#[derive(Debug)]
pub struct ProcessLock {
    path: PathBuf,
    file: Option<File>,
}

impl ProcessLock {
    /// Attempts to acquire the default cross-process simulation lock (`~/.cache/alphafind/sim_runner.lock`)
    /// without blocking. Fails closed immediately if another process holds the lock.
    pub fn try_acquire() -> Result<Self> {
        let lock_path = get_default_lock_path();
        Self::try_acquire_at(&lock_path)
    }

    /// Attempts to acquire an advisory lock at `path` without blocking.
    ///
    /// Returns `Err` with an explanation if another process currently holds the lock.
    pub fn try_acquire_at<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for lock file at '{}'",
                    parent.display()
                )
            })?;
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path_ref)
            .with_context(|| {
                format!(
                    "Failed to open or create lock file at '{}'",
                    path_ref.display()
                )
            })?;

        #[cfg(unix)]
        {
            let _ = fs::set_permissions(path_ref, fs::Permissions::from_mode(0o600));

            let fd = file.as_raw_fd();
            let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            if ret != 0 {
                let err = std::io::Error::last_os_error();
                let raw_os_err = err.raw_os_error().unwrap_or(0);
                if raw_os_err == libc::EWOULDBLOCK || raw_os_err == libc::EAGAIN {
                    return Err(anyhow!(
                        "Simulation runner lock is held by another process at '{}'. AGENTS.md policy limits concurrent simulation workers per account to 3 across all processes.",
                        path_ref.display()
                    ));
                } else {
                    return Err(anyhow!(
                        "Failed to acquire advisory flock on '{}': {}",
                        path_ref.display(),
                        err
                    ));
                }
            }
        }

        // Record PID and timestamp into lock file for observability
        let pid = std::process::id();
        let timestamp = chrono_or_epoch();
        let lock_meta = format!("pid={}\ntimestamp={}\n", pid, timestamp);
        let _ = file.set_len(0);
        let _ = file.write_all(lock_meta.as_bytes());
        let _ = file.flush();

        Ok(Self {
            path: path_ref.to_path_buf(),
            file: Some(file),
        })
    }

    /// Attempts to acquire the lock at `path`, polling repeatedly until `timeout` expires.
    pub fn acquire_with_timeout<P: AsRef<Path>>(
        path: P,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<Self> {
        let start = Instant::now();
        let path_ref = path.as_ref();

        loop {
            match Self::try_acquire_at(path_ref) {
                Ok(lock) => return Ok(lock),
                Err(e) => {
                    if start.elapsed() >= timeout {
                        return Err(anyhow!(
                            "Timed out after {:?} waiting for simulation lock at '{}': {}",
                            timeout,
                            path_ref.display(),
                            e
                        ));
                    }
                    std::thread::sleep(poll_interval);
                }
            }
        }
    }

    /// Explicitly releases the lock.
    pub fn release(mut self) -> Result<()> {
        self.unlock_internal()
    }

    /// Path to the locked file.
    pub fn lock_path(&self) -> &Path {
        &self.path
    }

    /// Whether the lock file descriptor is currently open.
    pub fn is_held(&self) -> bool {
        self.file.is_some()
    }

    fn unlock_internal(&mut self) -> Result<()> {
        if let Some(file) = self.file.take() {
            #[cfg(unix)]
            {
                let fd = file.as_raw_fd();
                let ret = unsafe { libc::flock(fd, libc::LOCK_UN) };
                if ret != 0 {
                    let err = std::io::Error::last_os_error();
                    return Err(anyhow!(
                        "Failed to release advisory flock on '{}': {}",
                        self.path.display(),
                        err
                    ));
                }
            }
            drop(file);
        }
        Ok(())
    }
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        let _ = self.unlock_internal();
    }
}

fn chrono_or_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
