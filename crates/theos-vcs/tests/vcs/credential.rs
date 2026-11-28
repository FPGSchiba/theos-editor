//! Credentials reach git only through the credential helper: a local HTTP server asks for them.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use theos_vcs::Cancel;
use theos_vcs::push::{PushError, PushTarget, push};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A server that answers 401 once, records the next `Authorization` header and answers 403.
struct Server {
    port: u16,
    seen: Arc<Mutex<Option<String>>>,
    thread: JoinHandle<()>,
}

fn reply(stream: &mut TcpStream, status: &str, extra: &str) {
    let text =
        format!("HTTP/1.1 {status}\r\n{extra}Content-Length: 0\r\nConnection: close\r\n\r\n");
    // The client may have gone; there is nothing to do about it in a test server.
    drop(stream.write_all(text.as_bytes()));
}

/// Reads one request head; returns its text, `None` on a timeout or a closed connection.
fn read_head(stream: &mut TcpStream) -> Option<String> {
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).ok()? == 0 {
            return None;
        }
        head.push(byte[0]);
    }
    Some(String::from_utf8_lossy(&head).into_owned())
}

impl Server {
    fn start() -> Result<Server> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let seen = Arc::new(Mutex::new(None));
        let recorded = Arc::clone(&seen);
        let thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            let mut requests = 0;
            while requests < 2 && Instant::now() < deadline {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                };
                drop(stream.set_nonblocking(false));
                let Some(head) = read_head(&mut stream) else {
                    continue;
                };
                requests += 1;
                if requests == 1 {
                    reply(
                        &mut stream,
                        "401 Unauthorized",
                        "WWW-Authenticate: Basic realm=\"t\"\r\n",
                    );
                } else {
                    let header = head
                        .lines()
                        .find_map(|line| line.strip_prefix("Authorization: "))
                        .map(str::to_owned);
                    if let Ok(mut slot) = recorded.lock() {
                        *slot = header;
                    }
                    reply(&mut stream, "403 Forbidden", "");
                }
            }
        });
        Ok(Server { port, seen, thread })
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/r.git", self.port)
    }

    /// Waits for the thread and returns the recorded `Authorization` header.
    fn finish(self) -> Result<Option<String>> {
        self.thread.join().map_err(|_| "server thread panicked")?;
        let seen = self.seen.lock().map_err(|_| "poisoned")?.clone();
        Ok(seen)
    }
}

fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(TABLE[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn target(url: &str) -> PushTarget {
    PushTarget {
        remote: url.to_owned(),
        branch: "main".to_owned(),
        set_upstream: false,
    }
}

fn repo_with_commit(sandbox: &Sandbox) -> Result<std::path::PathBuf> {
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("a.md"), "one\n")?;
    sandbox.commit(&top, "first")?;
    sandbox.run(&top, &["config", "credential.useHttpPath", "false"])?;
    Ok(top)
}

// AC: CE-22.3
#[test]
fn the_credential_helper_supplies_the_credentials() -> Result {
    let sandbox = Sandbox::new()?;
    let top = repo_with_commit(&sandbox)?;
    let marker = sandbox.path().join("helper-marker");
    let marker_text = marker.to_str().ok_or("path")?.replace('\\', "/");
    let helper = format!(
        "!f() {{ echo username=helper-user; echo password=helper-token; touch '{marker_text}'; }}; f"
    );
    sandbox.run(&top, &["config", "credential.helper", &helper])?;
    let server = Server::start()?;

    let result = push(
        &sandbox.git()?,
        &top,
        &target(&server.url()),
        &Cancel::new(),
    );

    assert!(
        matches!(result, Err(PushError::Failed { .. })),
        "{result:?}"
    );
    assert!(marker.exists(), "the helper did not run");
    let expected = format!("Basic {}", base64(b"helper-user:helper-token"));
    assert_eq!(server.finish()?, Some(expected));
    Ok(())
}

// AC: CE-22.3
#[test]
fn without_a_helper_git_fails_fast_and_never_runs_askpass() -> Result {
    let sandbox = Sandbox::new()?;
    let top = repo_with_commit(&sandbox)?;
    let marker = sandbox.path().join("askpass-marker");
    let marker_text = marker.to_str().ok_or("path")?.replace('\\', "/");
    let script = sandbox.path().join("askpass.sh");
    Sandbox::write(
        &script,
        &format!("#!/bin/sh\ntouch '{marker_text}'\necho secret\n"),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
    }
    let script_text = script.to_str().ok_or("path")?;
    let git = sandbox
        .git()?
        .with_env(&[("GIT_ASKPASS", script_text), ("SSH_ASKPASS", script_text)]);
    let server = Server::start()?;

    let started = Instant::now();
    let result = push(&git, &top, &target(&server.url()), &Cancel::new());

    assert!(started.elapsed() < Duration::from_secs(10));
    match result {
        Err(PushError::Failed { message }) => assert_ne!(message, ""),
        other => return Err(format!("{other:?}").into()),
    }
    assert!(!marker.exists(), "askpass ran");
    // Git never retried with a credential, so the server may still wait: do not join it.
    Ok(())
}
