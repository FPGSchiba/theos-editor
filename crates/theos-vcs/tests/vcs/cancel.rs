//! Cancelling git: the whole process tree stops, a cancelled flag starts nothing, large answers
//! are still read while the flag is watched, and failures are still reported.

use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, Instant};

use theos_vcs::{Cancel, GitError};

use crate::common::Sandbox;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The operating system, for failure messages.
const OS: &str = std::env::consts::OS;

/// The arguments of a git alias that starts `sh` and `sleep`, then writes `marker` after 2 s.
fn slow_alias(marker: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let marker = marker.to_str().ok_or("marker path")?.replace('\\', "/");
    Ok(vec![
        "-c".to_owned(),
        format!("alias.slow=!sleep 2 && echo done > '{marker}'"),
        "slow".to_owned(),
    ])
}

fn os_args(list: &[String]) -> Vec<&OsStr> {
    list.iter().map(OsStr::new).collect()
}

#[test]
fn a_cancel_stops_the_whole_tree() -> TestResult {
    let sandbox = Sandbox::new()?;
    let git = sandbox.git()?;
    let marker = sandbox.path().join("marker");
    let args = slow_alias(&marker)?;
    let cancel = Cancel::new();
    let (result, cancelled_at) = std::thread::scope(|scope| {
        let run = scope.spawn(|| {
            let result = git.run_cancellable(&os_args(&args), sandbox.path(), None, &[], &cancel);
            (result, Instant::now())
        });
        std::thread::sleep(Duration::from_millis(300));
        let cancelled_at = Instant::now();
        cancel.cancel();
        (run.join(), cancelled_at)
    });
    let (result, returned_at) = result.map_err(|_| "the run panicked")?;
    assert!(
        matches!(result, Err(GitError::Cancelled { .. })),
        "{OS}: expected Cancelled, got {result:?}"
    );
    let delay = returned_at.duration_since(cancelled_at);
    assert!(
        delay < Duration::from_secs(1),
        "{OS}: Cancelled came {delay:?} after the cancel"
    );
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        !marker.exists(),
        "{OS}: a process of git's tree lived on and wrote the marker"
    );
    Ok(())
}

#[test]
fn a_cancel_before_the_start_starts_nothing() -> TestResult {
    let sandbox = Sandbox::new()?;
    let git = sandbox.git()?;
    let marker = sandbox.path().join("marker");
    let args = slow_alias(&marker)?;
    let cancel = Cancel::new();
    cancel.cancel();
    let started = Instant::now();
    let result = git.run_cancellable(&os_args(&args), sandbox.path(), None, &[], &cancel);
    match &result {
        Err(GitError::Cancelled { args }) => {
            assert_eq!(args.last().map(String::as_str), Some("slow"));
        }
        other => return Err(format!("{OS}: expected Cancelled, got {other:?}").into()),
    }
    assert!(started.elapsed() < Duration::from_millis(100), "{OS}");
    std::thread::sleep(Duration::from_millis(2500));
    assert!(!marker.exists(), "{OS}: git was started");
    Ok(())
}

#[test]
fn a_large_answer_is_read_while_cancel_is_watched() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let git = sandbox.git()?;
    let cancel = Cancel::new();
    // 64 KiB asked for 64 times: an answer of 4 MiB, far larger than a pipe.
    let blob: Vec<u8> = (0..64 * 1024).map(|i| b"abcdefgh\n"[i % 9]).collect();
    let id = git.run_cancellable(
        &os_args(&["hash-object".into(), "-w".into(), "--stdin".into()]),
        &repo,
        Some(&blob),
        &[],
        &cancel,
    )?;
    let id = String::from_utf8(id)?;
    let asks = id.repeat(64);
    let answer = git.run_cancellable(
        &os_args(&["cat-file".into(), "--batch".into()]),
        &repo,
        Some(asks.as_bytes()),
        &[],
        &cancel,
    )?;
    let header = format!("{} blob {}\n", id.trim(), blob.len());
    let mut expected = Vec::new();
    for _ in 0..64 {
        expected.extend_from_slice(header.as_bytes());
        expected.extend_from_slice(&blob);
        expected.push(b'\n');
    }
    assert_eq!(answer.len(), expected.len(), "{OS}");
    assert!(answer == expected, "{OS}: the answer differs");
    Ok(())
}

#[test]
fn run_still_reports_failures() -> TestResult {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let result = sandbox.git()?.run(
        &os_args(&["rev-parse".into(), "--verify".into(), "nope".into()]),
        &repo,
        None,
        &[],
    );
    match result {
        Err(GitError::Failed { args, code, stderr }) => {
            assert_eq!(args, ["rev-parse", "--verify", "nope"]);
            assert_eq!(code, Some(128));
            assert!(stderr.contains("Needed a single revision"), "{stderr}");
        }
        other => return Err(format!("{OS}: unexpected {other:?}").into()),
    }
    Ok(())
}
