//! Empirical stress tests for Milestone M4 Concurrency & Lock Contention:
//! - Multi-threaded and multi-process heavy contention stress testing.
//! - Strict fail-closed verification of non-blocking acquisition (`try_acquire_at`).
//! - Timed acquisition queueing, sequential handover, and clean timeout expiry (`acquire_with_timeout`).
//! - Stale lock recovery after abrupt process termination (`SIGKILL`).
//! - Cross-language mutual exclusion between Rust `ProcessLock` and Python `fcntl.flock`.
//! - Screener integration mutual exclusion gatekeeper.
//! - Deeply nested directory creation, `0600` permission enforcement, and corruption resistance.

use alphafind::client::BrainClient;
use alphafind::concurrency_lock::ProcessLock;
use alphafind::models::CandidateAlpha;
use alphafind::screener::{AccountWorker, MultiAccountScreener};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn unique_temp_lock_path(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir()
        .join(format!(
            "af_stress_{}_{}_{}",
            label,
            std::process::id(),
            nonce
        ))
        .join("sim_runner.lock")
}

// ==============================================================================
// 1. Multi-Threaded High-Contention Race Test
// ==============================================================================

#[test]
fn test_stress_multithreaded_high_contention_race() {
    let lock_path = unique_temp_lock_path("mt_race");
    let thread_count = 32;
    let rounds = 5;

    for round in 1..=rounds {
        let barrier = Arc::new(Barrier::new(thread_count));
        let success_count = Arc::new(AtomicUsize::new(0));
        let contended_count = Arc::new(AtomicUsize::new(0));
        let other_err_count = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::with_capacity(thread_count);

        for _ in 0..thread_count {
            let b = barrier.clone();
            let path = lock_path.clone();
            let succ = success_count.clone();
            let cont = contended_count.clone();
            let oth = other_err_count.clone();

            handles.push(std::thread::spawn(move || {
                // Wait for all threads to reach the starting gate
                b.wait();

                // Simultaneous burst of try_acquire_at
                match ProcessLock::try_acquire_at(&path) {
                    Ok(lock) => {
                        succ.fetch_add(1, Ordering::SeqCst);
                        assert!(lock.is_held());
                        // Hold lock briefly to force all other threads to contend
                        std::thread::sleep(Duration::from_millis(25));
                        drop(lock); // Explicit RAII drop
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("held by another process") {
                            cont.fetch_add(1, Ordering::SeqCst);
                        } else {
                            oth.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            }));
        }

        for h in handles {
            h.join().expect("Thread must not panic");
        }

        let s = success_count.load(Ordering::SeqCst);
        let c = contended_count.load(Ordering::SeqCst);
        let o = other_err_count.load(Ordering::SeqCst);

        assert_eq!(
            s, 1,
            "Round {round}: Exactly ONE thread must acquire the lock (got {s})"
        );
        assert_eq!(
            c,
            thread_count - 1,
            "Round {round}: All other {c} threads must fail closed with contention error"
        );
        assert_eq!(
            o, 0,
            "Round {round}: Zero unexpected errors permitted (got {o})"
        );
    }

    if let Some(parent) = lock_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

// ==============================================================================
// 2. Sequential Handover Under Timed Acquisition Queue
// ==============================================================================

#[test]
fn test_stress_timeout_queueing_and_sequential_handoff() {
    let lock_path = unique_temp_lock_path("timeout_handoff");
    let worker_count = 8;
    let barrier = Arc::new(Barrier::new(worker_count + 1));
    let active_in_critical = Arc::new(AtomicUsize::new(0));
    let completed_count = Arc::new(AtomicUsize::new(0));

    // Acquire lock initially so all workers queue up
    let initial_lock =
        ProcessLock::try_acquire_at(&lock_path).expect("Initial lock acquisition must succeed");
    assert!(initial_lock.is_held());

    let mut handles = Vec::with_capacity(worker_count);

    for worker_id in 0..worker_count {
        let b = barrier.clone();
        let path = lock_path.clone();
        let active = active_in_critical.clone();
        let completed = completed_count.clone();

        handles.push(std::thread::spawn(move || {
            b.wait(); // Synchronize start with main thread

            // Attempt acquisition with generous timeout (4000ms)
            let lock = ProcessLock::acquire_with_timeout(
                &path,
                Duration::from_millis(4000),
                Duration::from_millis(10),
            )
            .unwrap_or_else(|e| {
                panic!("Worker {worker_id} failed to acquire lock: {e}");
            });

            // Critical section verification
            let prev = active.fetch_add(1, Ordering::SeqCst);
            assert_eq!(
                prev, 0,
                "Mutual exclusion violation! Another thread was active inside critical section: worker {worker_id}"
            );

            assert!(lock.is_held());
            std::thread::sleep(Duration::from_millis(20));

            active.fetch_sub(1, Ordering::SeqCst);
            completed.fetch_add(1, Ordering::SeqCst);
            drop(lock);
        }));
    }

    // Synchronize barrier with workers
    barrier.wait();

    // Hold lock for 50ms while workers start polling, then release
    std::thread::sleep(Duration::from_millis(50));
    drop(initial_lock);

    for h in handles {
        h.join().expect("Worker thread must finish cleanly");
    }

    assert_eq!(
        completed_count.load(Ordering::SeqCst),
        worker_count,
        "All workers must have successfully completed sequential critical section"
    );

    if let Some(parent) = lock_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

// ==============================================================================
// 3. Timed Acquisition Clean Expiration Under Continuous Contention
// ==============================================================================

#[test]
fn test_stress_timeout_expiration_under_continuous_contention() {
    let lock_path = unique_temp_lock_path("timeout_expire");

    // Hold the lock continuously in the test runner
    let held_lock =
        ProcessLock::try_acquire_at(&lock_path).expect("Lock must be acquired by holder");
    assert!(held_lock.is_held());

    let thread_count = 6;
    let barrier = Arc::new(Barrier::new(thread_count));
    let timeout_duration = Duration::from_millis(80);
    let mut handles = Vec::with_capacity(thread_count);

    for i in 0..thread_count {
        let b = barrier.clone();
        let path = lock_path.clone();

        handles.push(std::thread::spawn(move || {
            b.wait();
            let start = Instant::now();
            let res = ProcessLock::acquire_with_timeout(
                &path,
                timeout_duration,
                Duration::from_millis(15),
            );
            let elapsed = start.elapsed();

            assert!(
                res.is_err(),
                "Thread {i} must fail with timeout error when lock is held continuously"
            );
            let err_msg = res.err().unwrap().to_string();
            assert!(
                err_msg.contains("Timed out"),
                "Thread {i} error message must indicate timeout: got '{err_msg}'"
            );
            assert!(
                elapsed >= Duration::from_millis(70),
                "Thread {i} elapsed ({:?}) must reflect requested timeout duration ({:?})",
                elapsed,
                timeout_duration
            );
        }));
    }

    for h in handles {
        h.join().expect("Thread must not panic");
    }

    // After all threads have cleanly timed out, release the lock
    drop(held_lock);

    // Now acquisition must succeed immediately
    let final_lock = ProcessLock::try_acquire_at(&lock_path)
        .expect("Lock acquisition after holder release must succeed");
    assert!(final_lock.is_held());
    drop(final_lock);

    if let Some(parent) = lock_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

// ==============================================================================
// 4. Stale Lock Recovery After Abnormal Termination (SIGKILL)
// ==============================================================================

#[test]
fn test_stress_stale_lock_recovery_after_sigkill() {
    let lock_path = unique_temp_lock_path("sigkill_recovery");
    if let Some(parent) = lock_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Spawn a child Python process that acquires flock and blocks indefinitely
    let python_code = format!(
        r#"
import fcntl, time, os, sys
path = r"{}"
f = open(path, "a+")
fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
f.seek(0)
f.truncate()
f.write(f"pid={{os.getpid()}}\ntimestamp=123456789\n")
f.flush()
sys.stdout.write("ACQUIRED\n")
sys.stdout.flush()
time.sleep(60)
"#,
        lock_path.display()
    );

    let mut child = Command::new("python3")
        .arg("-c")
        .arg(&python_code)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn python child process for flock test");

    // Wait until child outputs ACQUIRED
    use std::io::BufRead;
    let stdout = child.stdout.take().expect("Child stdout must be available");
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("Failed to read ACQUIRED from child");
    assert!(
        line.contains("ACQUIRED"),
        "Child process must acquire flock first: got '{line}'"
    );

    // Verify parent Rust process cannot acquire while child is alive -> must fail closed
    let cont_res = ProcessLock::try_acquire_at(&lock_path);
    assert!(
        cont_res.is_err(),
        "Rust process must be blocked from acquiring lock while child holds it"
    );
    let err_str = cont_res.err().unwrap().to_string();
    assert!(
        err_str.contains("held by another process"),
        "Must fail closed with contention error: got '{err_str}'"
    );

    // Kill child process abruptly with SIGKILL (kill -9)
    #[cfg(unix)]
    unsafe {
        libc::kill(child.id() as i32, libc::SIGKILL);
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
    let _ = child.wait();

    // Verify lock file still physically exists with the dead child's PID
    assert!(lock_path.exists(), "Lock file must remain on disk");
    let dead_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        dead_content.contains("timestamp=123456789"),
        "Lock file still contains dead process metadata before recovery"
    );

    // Immediately attempt to acquire lock in Rust -> OS kernel automatically released flock on child exit!
    let recovered_lock = ProcessLock::try_acquire_at(&lock_path)
        .expect("Stale lock must be cleanly recovered without deadlock or manual file deletion");
    assert!(recovered_lock.is_held());

    // Verify file content was cleanly overwritten with new PID and timestamp
    let new_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        new_content.contains(&format!("pid={}", std::process::id())),
        "Lock file must be refreshed with current process PID: got '{new_content}'"
    );
    assert!(
        !new_content.contains("123456789"),
        "Stale timestamp must be completely overwritten: got '{new_content}'"
    );

    drop(recovered_lock);

    if let Some(parent) = lock_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

// ==============================================================================
// 5. Cross-Language Mutual Exclusion (Rust ProcessLock vs Python CrossProcessLock)
// ==============================================================================

#[test]
fn test_stress_cross_language_python_rust_mutual_exclusion() {
    let lock_path = unique_temp_lock_path("cross_lang");

    // Step A: Rust acquires the lock
    let rust_lock = ProcessLock::try_acquire_at(&lock_path).expect("Rust must acquire fresh lock");
    assert!(rust_lock.is_held());

    // Run Python test: try acquiring via fcntl.LOCK_EX | fcntl.LOCK_NB
    let py_check_code = format!(
        r#"
import fcntl, sys
path = r"{}"
try:
    f = open(path, "a+")
    fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    sys.exit(0) # Acquired (should not happen!)
except (IOError, OSError):
    sys.exit(42) # Contended (expected)
"#,
        lock_path.display()
    );

    let py_res = Command::new("python3")
        .arg("-c")
        .arg(&py_check_code)
        .status()
        .expect("Failed to run python lock check");

    assert_eq!(
        py_res.code(),
        Some(42),
        "Python must detect Rust lock and fail closed with exit code 42"
    );

    // Step B: Drop Rust lock
    drop(rust_lock);

    // Step C: Python acquires lock and holds it for 500ms
    let py_hold_code = format!(
        r#"
import fcntl, time, sys
path = r"{}"
f = open(path, "a+")
fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
sys.stdout.write("PY_LOCKED\n")
sys.stdout.flush()
time.sleep(0.5)
fcntl.flock(f, fcntl.LOCK_UN)
sys.exit(0)
"#,
        lock_path.display()
    );

    let mut py_holder = Command::new("python3")
        .arg("-c")
        .arg(&py_hold_code)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn python lock holder");

    use std::io::BufRead;
    let stdout = py_holder.stdout.take().unwrap();
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains("PY_LOCKED"));

    // Rust attempts non-blocking acquire while Python holds it -> must fail closed
    let rust_fail_res = ProcessLock::try_acquire_at(&lock_path);
    assert!(
        rust_fail_res.is_err(),
        "Rust ProcessLock must fail closed while Python holds the flock"
    );
    assert!(rust_fail_res
        .err()
        .unwrap()
        .to_string()
        .contains("held by another process"));

    // Wait for Python to release and exit
    let py_exit = py_holder.wait().unwrap();
    assert!(py_exit.success());

    // Rust acquires immediately after Python releases
    let rust_after = ProcessLock::try_acquire_at(&lock_path)
        .expect("Rust must acquire lock after Python releases it");
    assert!(rust_after.is_held());
    drop(rust_after);

    if let Some(parent) = lock_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

// ==============================================================================
// 6. Screener Double-Invocation Gatekeeper
// ==============================================================================

#[tokio::test]
async fn test_stress_screener_double_invocation_mutual_exclusion() {
    let lock_path = unique_temp_lock_path("screener_exclusion");

    // Manually acquire the lock to simulate an active screening run
    let active_lock =
        ProcessLock::try_acquire_at(&lock_path).expect("Initial lock acquisition must succeed");
    assert!(active_lock.is_held());

    // Construct a mock MultiAccountScreener
    let client = BrainClient::new("screener_stress@example.com", "dummy_pass")
        .expect("Client constructor must succeed");
    let screener = MultiAccountScreener {
        main_client: client.clone(),
        accounts: vec![AccountWorker {
            client,
            role: "MAIN".to_string(),
            is_main: true,
            semaphore: Arc::new(tokio::sync::Semaphore::new(3)),
        }],
        workers_per_account: 3,
        transfer_lock: Arc::new(tokio::sync::Mutex::new(())),
    };

    let cand = CandidateAlpha {
        name: "STRESS_CAND_01".to_string(),
        expression: "rank(close)".to_string(),
        universe: "TOP1000".to_string(),
        decay: 10,
        neutralization: "SUBINDUSTRY".to_string(),
        truncation: 0.05,
        pillar: "RiskMetrics".to_string(),
    };

    // Attempting to run screener on the held lock path must fail closed immediately
    let res = screener
        .screen_batch_at_lock(vec![cand], 1.0, false, Some(&lock_path))
        .await;

    assert!(
        res.is_err(),
        "Screener must strictly fail closed when another process holds the simulation lock"
    );
    let err_msg = res.err().unwrap().to_string();
    assert!(
        err_msg.contains("unable to acquire cross-process simulation lock")
            || err_msg.contains("held by another process"),
        "Error message must clearly identify lock acquisition failure: got '{err_msg}'"
    );

    // Release the active lock
    drop(active_lock);

    // Verify empty candidate batch returns Ok without acquiring lock
    let empty_res = screener
        .screen_batch_at_lock::<&std::path::Path>(Vec::new(), 1.0, false, Some(&lock_path))
        .await;
    assert!(empty_res.is_ok());
    assert!(empty_res.unwrap().is_empty());

    if let Some(parent) = lock_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

// ==============================================================================
// 7. Nested Directory Creation & 0600 Permissions Invariant
// ==============================================================================

#[test]
fn test_stress_nested_directory_creation_and_permissions() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let deep_dir = std::env::temp_dir()
        .join(format!("af_deep_{}_{}", std::process::id(), nonce))
        .join("sub1")
        .join("sub2")
        .join("sub3");
    let deep_lock_path = deep_dir.join("nested.lock");

    assert!(
        !deep_dir.exists(),
        "Deep directory must not exist prior to test"
    );

    let lock = ProcessLock::try_acquire_at(&deep_lock_path)
        .expect("ProcessLock must automatically create all parent directories");
    assert!(lock.is_held());
    assert!(deep_lock_path.exists());

    #[cfg(unix)]
    {
        let metadata = fs::metadata(&deep_lock_path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "Lock file must be created with strict 0600 permissions (got {mode:o})"
        );
    }

    drop(lock);

    let root_parent = deep_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let _ = fs::remove_dir_all(root_parent);
}

// ==============================================================================
// 8. Corrupted Lockfile Recovery & Overwrite Cleanliness
// ==============================================================================

#[test]
fn test_stress_corrupted_lockfile_recovery_and_clean_overwrite() {
    let lock_path = unique_temp_lock_path("corrupt_recovery");
    if let Some(parent) = lock_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Write 16KB of corrupted binary data into the lock file path
    let junk_data = [0xDE, 0xAD, 0xBE, 0xEF];
    let repeated_junk: Vec<u8> = junk_data.iter().cycle().take(16384).cloned().collect();
    fs::write(&lock_path, &repeated_junk).expect("Must write junk file");
    assert_eq!(fs::metadata(&lock_path).unwrap().len(), 16384);

    // Acquire lock: must truncate corrupted data and write fresh valid metadata
    let lock = ProcessLock::try_acquire_at(&lock_path)
        .expect("Corrupted lockfile without active flock must be cleanly overwritten");
    assert!(lock.is_held());

    let content = fs::read_to_string(&lock_path).expect("Must read lock content");
    assert!(
        content.contains(&format!("pid={}", std::process::id())),
        "Refreshed lockfile must contain current PID: got '{content}'"
    );
    assert!(
        content.contains("timestamp="),
        "Refreshed lockfile must contain timestamp: got '{content}'"
    );
    assert!(
        content.len() < 80,
        "Refreshed lockfile must be truncated cleanly (length is {}, expected < 80)",
        content.len()
    );

    drop(lock);

    if let Some(parent) = lock_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

// ==============================================================================
// 9. Multi-Process Simultaneous Burst Contention
// ==============================================================================

#[test]
fn test_stress_multiprocess_high_contention_burst() {
    let lock_path = unique_temp_lock_path("proc_burst");
    let trigger_path = lock_path.parent().unwrap().join("trigger.go");
    if let Some(parent) = lock_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let process_count = 10;
    let py_script = r#"
import fcntl, sys, time, os
lock_path = sys.argv[1]
trigger_path = sys.argv[2]
while not os.path.exists(trigger_path):
    time.sleep(0.001)
try:
    f = open(lock_path, "a+")
    fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    time.sleep(0.4)
    fcntl.flock(f, fcntl.LOCK_UN)
    sys.exit(0)
except (IOError, OSError):
    sys.exit(42)
"#;

    let mut children = Vec::with_capacity(process_count);
    for _ in 0..process_count {
        let child = Command::new("python3")
            .arg("-c")
            .arg(py_script)
            .arg(&lock_path)
            .arg(&trigger_path)
            .spawn()
            .expect("Must spawn python process");
        children.push(child);
    }

    // Give children 100ms to initialize and enter spin-wait loop
    std::thread::sleep(Duration::from_millis(100));

    // Create trigger file to release all 10 processes simultaneously
    fs::write(&trigger_path, b"GO").expect("Must write trigger file");

    let mut exit_0_count = 0;
    let mut exit_42_count = 0;

    for mut c in children {
        let status = c.wait().expect("Child process wait must succeed");
        match status.code() {
            Some(0) => exit_0_count += 1,
            Some(42) => exit_42_count += 1,
            other => panic!("Unexpected child exit status: {:?}", other),
        }
    }

    assert_eq!(
        exit_0_count, 1,
        "Exactly ONE OS process must acquire flock during burst race (got {exit_0_count})"
    );
    assert_eq!(
        exit_42_count,
        process_count - 1,
        "All other OS processes must receive EAGAIN/EWOULDBLOCK and exit with 42 (got {exit_42_count})"
    );

    if let Some(parent) = lock_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

// ==============================================================================
// 10. Invalid Path Fails Closed Gracefully
// ==============================================================================

#[test]
fn test_stress_invalid_path_fails_closed_gracefully() {
    let temp_file = unique_temp_lock_path("invalid_path_blocker");
    if let Some(parent) = temp_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&temp_file, b"I am a file, not a directory").unwrap();

    let invalid_lock_path = temp_file.join("impossible_sub_dir").join("sim.lock");
    let res = ProcessLock::try_acquire_at(&invalid_lock_path);

    assert!(res.is_err(), "Must fail closed for impossible path");
    let err_str = res.err().unwrap().to_string();
    assert!(
        err_str.contains("Failed to create parent directory")
            || err_str.contains("Failed to open or create lock file"),
        "Error message must specify path creation failure: got '{err_str}'"
    );

    if let Some(parent) = temp_file.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}
