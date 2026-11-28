# P47 · Git write — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (CLAUDE.md) to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `theos-vcs` stages, unstages, commits with trailers, fetches, merges, pushes and reads
the bytes of a checkout; `theos-studio` does the same by record under the repo lock, with an
undoable `Discard`, a commit and a pull that judge the staged snapshot first (CE-22.1), and a push
whose errors carry git's message whole while credentials go only through the helper (CE-22.3).

**Architecture:** two layers, as in P46.
- `theos-vcs` (bytes, git spelling, no record knowledge, no lock): `Output` and `Git::output`, one
  module of write options, `stage`, `checkout_bytes`, `commit`, `remote` (environment, config
  reads, fetch), `push`, `merge`, `housekeeping`.
- `theos-studio` (records, the repo lock, the checks): `git_write::{stage, commit, pull, push}`
  and the `Discard` command in the command layer.

**Tech stack:** Rust 2024 (toolchain from `rust-toolchain.toml`), git ≥ 2.38 on the PATH (tests use
the real git). No new crate: `thiserror`, `tracing`, `tempfile`, std (`std::net::TcpListener` for
the credential test's HTTP server).

**Spec:** `docs/specs/phases/P47-git-write.md` (approved in #70). Read it with this plan; section
numbers below (§5.x) are the spec's. Interfaces it builds on: P05 spec §5.2, §6; P06 spec §5.3–5.9;
P46 spec §5.3–5.8; `crates/theos-vcs/README.md`.

**Order:** Tasks 1–2 first (every later vcs task runs git through them). Tasks 3–8 (vcs) in any
order after 2. Task 9 before 10–13. Task 10 needs 4; 11 needs 5; 12 needs 7 and 11; 13 needs 6.
Task 14 last.

## Global Constraints

The umbrella's global constraints G1–G16, verbatim (`docs/specs/00-umbrella.md` §2):

- **G1** Repositories: `theos-editor` (this repo) holds the editor and the map generator
  (`theos-map-format`, `theos-maps`). `tower-of-theos` holds the content project, its map recipes
  and the blockout generator. `Theos-Engine` is never a dependency, except behind the cargo
  feature `engine`, which no phase here builds; the engine may depend on the map crates.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS on Apple silicon
  are tested weekly, on demand (a label on the PR) and before every release, and macOS on Intel
  on demand and before every release (`docs/gate.md` §1).
- **G3** Rust: stable toolchain pinned in `rust-toolchain.toml`, edition 2024, `unsafe_code`
  forbidden, `Cargo.lock` committed, all builds `--locked`.
- **G4** Crates: `theos-content`, `theos-vcs`, `theos-studio`, `theos-map-format`, `theos-maps`
  (libs); `theos`, `theos-mcp`, `theos-editor` (bins); `xtask`. Paths `crates/<name>`, `xtask/`. UI in `editor-ui/`
  (Svelte 5, TypeScript, pnpm).
- **G5** Content files follow `docs/specs/data-model.md`; the restricted YAML parser is ours;
  any edit leaves untouched bytes identical.
- **G6** Git only through the `git` command line; credentials only through the user's
  credential helper.
- **G7** MCP: stdio only, `rmcp`, tool names `^[a-z][a-z0-9_]*$`, agents only propose.
- **G8** `never-confirm`, `no-overshoot`, `inworld-vocab` are always blocking; nothing lowers them.
- **G9** Text of a truth with target `never` never appears in an LLM prompt.
- **G10** Binaries never enter git; references are `path@sha256:<hex>` relative to the binaries
  folder.
- **G11** Generators only through `docs/generators.md`, the built-in map generator included; no
  generator-specific field in the data model; the UI works for any generator that produces a kind.
- **G12** Network only to ComfyUI and the configured LLM endpoint, both opt-in; no telemetry.
- **G13** UI uses `design/theos-ui/` and follows `docs/ui/`; shortcuts use Ctrl on Windows and
  Linux and Cmd on macOS, from one keymap.
- **G14** Settings in three layers (part default → user → project); secrets in the OS
  credential store.
- **G15** Every AC below has at least one test tagged with its id (`docs/gate.md` §4).
- **G16** Errors in library crates are typed (`thiserror`); `anyhow` only in binaries. Logging
  with `tracing`.

The phase's acceptance criteria, verbatim (umbrella §3 P47):

**CE-22 Git write and repo lock** (continued from P06)
- CE-22.1 Commit is blocked on errors in the staged snapshot and allowed with warnings.
- CE-22.3 Push errors show git's message; credentials only through the system helper.

Also binding (CLAUDE.md): engineering directives 1–4 (small modules with one job; existing tools
before custom code; change the approach when fixes leak; the gate catches shortcuts), no
`unwrap`/`expect` outside tests, public items documented, LF in files we write, no `#[allow]`,
tests tagged `// AC: <id>`, never `--no-verify` (also not in product code: theos never passes it
to git). Git in tests: the real `git`, `GIT_CONFIG_GLOBAL` on an empty file,
`GIT_CONFIG_NOSYSTEM=1`; remotes are local bare repositories; no test uses the network.

Commits: Conventional Commits with `Refs: <AC ids>` (when the task proves an AC), `Phase: P47` and
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Subagents never push.

## Review Focus

Inputs no task's main tests exercise that would bite a user first; each has its test in the
owning task:

1. A commit message typed in a Windows text field arrives with CRLF line ends → the commit holds
   LF lines and the trailers still come last (Task 5, `commit_message_with_crlf_is_stored_with_lf`).
2. A project in a subfolder of the repository whose changes outside the project are listed in
   `Changes.files` → stage and unstage take them; discard refuses them with `NotDiscardable`
   (Tasks 9, 10).
3. A hook that writes more than a pipe holds (64 KiB) to stderr and then refuses → the commit
   returns `Git` with the whole text, no deadlock (Task 5, `a_loud_refusing_hook_is_reported_whole`).
4. An upstream branch deleted on the remote (`[gone]`) → pull returns `NoUpstream` naming the
   branch, nothing merged (Task 12, `a_gone_upstream_is_no_upstream`).
5. Record paths with NFD characters or a case-only spelling difference on a case-insensitive disk
   → staging by record stages git's spelling (Task 9, `staging_uses_git_spelling`).

---

### Task 1: `Output`, `Git::output`, and a finished run keeps its result

**Files:**
- Modify: `crates/theos-vcs/src/git.rs`, `crates/theos-vcs/src/wait.rs`, `crates/theos-vcs/src/lib.rs`
- Test: `crates/theos-vcs/src/wait.rs` (unit), `crates/theos-vcs/tests/vcs/git.rs`

**Interfaces:**
- Produces:
  - `pub struct Output { pub code: Option<i32>, pub stdout: Vec<u8>, pub stderr: String }`
    (stderr lossily decoded and trimmed, as `GitError::Failed` has it), re-exported from the root;
  - `Git::output(&self, args: &[&OsStr], cwd: &Path, stdin: Option<&[u8]>, env: &[(&str, &str)],
    cancel: &Cancel) -> Result<Output, GitError>` — `Err` only for `Cancelled`, `Io`, `Output`;
  - `Output::success(&self) -> bool` (`code == Some(0)`).

- [ ] **Step 1: Write the failing tests:**
  - behaviour: `output` of `git rev-parse --verify --quiet nosuchref` in a fresh repo is `Ok` with
    `code == Some(1)` and empty stdout; `run` of the same is `GitError::Failed { code: Some(1) }`;
  - behaviour: `output` of `git hash-object --stdin` with stdin `b"x"` succeeds and stdout is the
    object id;
  - unit (`wait.rs`): the decision after a cancel is seen is a function of its own,
    `fn after_cancel(receiver: &Receiver<io::Result<ExitStatus>>, tree: Tree) -> io::Result<Waited>`;
    with a channel that already holds the status of a finished `git --version` (from
    `Command::status`) and the `Tree` of another finished `git --version`, it returns
    `Waited::Exited` with that status.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs` — expect FAIL.
- [ ] **Step 3: Implement.** `spawn_and_wait` returns an `Output`; `output` logs like
  `run_cancellable`; `run_cancellable` = `output` + map a non-success `Output` to `Failed`. In
  `wait`, after the loop breaks on a cancel, `after_cancel` checks `receiver.try_recv()` first: a
  status that has arrived is returned as `Exited` (today it returns `Cancelled`). Only a git still running is
  stopped. Keep the single spawn path (P46 ruling 3).
- [ ] **Step 4: Run** `cargo nextest run -p theos-vcs -p theos-studio -p theos` — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): git's whole answer, and a finished run keeps its result`.

### Task 2: Write options and `IndexLocked`

**Files:**
- Create: `crates/theos-vcs/src/write_args.rs`
- Modify: `crates/theos-vcs/src/git.rs` (`GitError::IndexLocked`), `crates/theos-vcs/src/lib.rs`
- Test: `crates/theos-vcs/src/write_args.rs` (unit), `crates/theos-vcs/tests/vcs/write.rs` (new,
  registered in `tests/vcs/main.rs`)

**Interfaces:**
- Produces:
  - `pub(crate) fn write_args<'a>(command: &'a str, flags: &[&'a str]) -> Vec<&'a OsStr>` — the
    options `--literal-pathspecs`, `-c color.ui=never`, `-c core.quotePath=false`,
    `-c gc.auto=0`, `-c maintenance.auto=false`, then `command`, then `flags` (no
    `--no-optional-locks`);
  - `pub(crate) fn write(git: &Git, top: &Path, command: &str, flags: &[&str], stdin: Option<&[u8]>,
    cancel: &Cancel) -> Result<Output, GitError>` running `write_args` through `Git::output`;
  - `pub(crate) fn checked(args: &[&OsStr], out: Output) -> Result<Output, GitError>`: success
    passes; a failure whose stderr contains `index.lock` is `GitError::IndexLocked { stderr }`;
    any other failure is `GitError::Failed`;
  - `GitError::IndexLocked { stderr: String }`, displayed as "another git is working in this
    repository (its index.lock exists): {stderr}".

- [ ] **Step 1: Write the failing tests:** the exact option list (unit, as P46's
  `read_args` test); behaviour: with `.git/index.lock` created by hand, `write(.., "add", ["-A"])`
  through `checked` is `IndexLocked` whose `stderr` names `index.lock`, and the file still exists
  afterwards.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs write` — expect FAIL.
- [ ] **Step 3: Implement** as in the Interfaces block.
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): one list of options for every git write`.

### Task 3: `stage` and `unstage`

**Files:**
- Create: `crates/theos-vcs/src/stage.rs` (`pub mod stage`)
- Test: `crates/theos-vcs/tests/vcs/stage.rs`

**Interfaces:**
- Consumes: Task 2.
- Produces: `pub fn stage(git: &Git, top: &Path, paths: &[&[u8]]) -> Result<(), GitError>`,
  `pub fn unstage(git: &Git, top: &Path, paths: &[&[u8]]) -> Result<(), GitError>`, and in
  `crates/theos-vcs/src/commit_id.rs` (re-exported from the root) `pub struct CommitId(pub String)`
  with `Display`, and `pub fn repo::head(git: &Git, top: &Path) -> Result<Option<CommitId>, GitError>`
  (`rev-parse -q --verify HEAD`; `None` before the first commit).

- [ ] **Step 1: Write the failing tests:**
  - a file renamed on disk (`i.md` → `i2.md`), `stage(["i.md", "i2.md"])` → `status` shows one
    `Entry::Renamed { staged: Renamed }`;
  - a new file staged then `unstage`d before the first commit → `Untracked`;
  - after a commit, a modified file staged then unstaged → `Changed { staged: Unchanged,
    unstaged: Modified }`;
  - paths `a b.md`, `*.md`, `x:y.md` (not on Windows: `cfg(not(windows))` with the reason in a
    comment), `-lead.md`, `ü.md` are staged literally and nothing else is (a file `ab.md` next
    to `*.md` stays untracked);
  - 3 000 paths of 20 characters (more than 32 767 characters on one command line) stage in one
    call;
  - an empty list runs no git (`stage` on a folder that is no repository with `[]` is `Ok`).
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs stage` — expect FAIL.
- [ ] **Step 3: Implement.** Paths NUL-joined on stdin. Stage: `add -A
  --pathspec-from-file=- --pathspec-file-nul`. Unstage: with a `repo::head`, `reset -q HEAD --pathspec-from-file=-
  --pathspec-file-nul`, without it `rm -q --cached --ignore-unmatch --pathspec-from-file=-
  --pathspec-file-nul`. All through `write` + `checked`, never cancellable (§5.2).
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): stage and unstage paths`.

### Task 4: `checkout_bytes`

**Files:**
- Modify: `crates/theos-vcs/src/read.rs`
- Test: `crates/theos-vcs/tests/vcs/read.rs`

**Interfaces:**
- Produces: `pub fn checkout_bytes(git: &Git, top: &Path, path: &[u8]) -> Result<Content, GitError>`
  in `theos_vcs::read` — the index's stage-0 entry, `Absent` when the index lacks it or holds it as
  intent-to-add, `NotAFile` for a link or submodule, else the bytes of
  `git cat-file --filters --path=<path> <object>`.

- [ ] **Step 1: Write the failing tests:** a blob `a\nb\n` with `core.autocrlf=true` gives
  `a\r\nb\r\n`; with `.gitattributes` `*.md eol=crlf` the same; without either the raw bytes; an
  untracked path is `Absent`; a staged edit gives the staged bytes, not `HEAD`'s.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs read` — expect FAIL.
- [ ] **Step 3: Implement** by reusing `index`'s object and mode resolution (`:0:<path>`,
  `ls-files -s`, intent-to-add) and replacing the `blobs` call with `cat-file --filters
  --path=<path> <object>` through `read_args`.
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): the bytes a checkout writes`.

### Task 5: `commit` with trailers (CE-22.1, vcs part)

**Files:**
- Create: `crates/theos-vcs/src/commit.rs` (`pub mod commit`)
- Test: `crates/theos-vcs/src/commit.rs` (unit), `crates/theos-vcs/tests/vcs/commit.rs`

**Interfaces:**
- Consumes: Tasks 2, 3 (`CommitId`, `repo::head`).
- Produces:
  - `pub struct Trailer { pub key: String, pub value: String }`;
  - `pub fn commit(git: &Git, top: &Path, message: &str, trailers: &[Trailer]) -> Result<CommitId, CommitRefused>`;
  - `#[non_exhaustive] pub enum CommitRefused { Message(String), Trailer(String), Git(#[from] GitError) }`;
  - `pub(crate) fn comment_char(message: &str) -> Option<char>` — the first of
    `['#', ';', '@', '!', '$', '%', '^', '&', '|', ':']` that starts no line.

- [ ] **Step 1: Write the failing tests** (behaviour ones tagged `// AC: CE-22.1`):
  - unit: `comment_char("a\n# b")` is `';'`; a message whose lines start with all ten is `None`;
  - a staged file, message `"F04: open the sky\n\n# hash line kept\n"`, trailer `Approved-by:
    Jann` → `git log -1 --format=%B` is exactly `"F04: open the sky\n\n# hash line kept\n\nApproved-by: Jann\n"`;
  - `commit_message_with_crlf_is_stored_with_lf` (Review Focus 1): `"a\r\n\r\nb\r\n"` is stored
    as `"a\n\nb\n"` with the trailer last;
  - an empty or whitespace-only message is `Message`; a trailer key `Approved by` or a value with
    `\n` is `Trailer`; nothing is committed in each case (`repo::head` unchanged);
  - a `pre-commit` hook script that prints `theos: 1 error` and exits 1 → `Git(Failed)` whose
    stderr contains `theos: 1 error`; `repo::head` unchanged;
  - `a_loud_refusing_hook_is_reported_whole` (Review Focus 3): the hook prints 100 000 bytes to
    stderr, then exits 1 → `Git(Failed)` with the whole text (length ≥ 100 000 after trim);
  - user's `core.commentChar=;` and `commit.verbose=true` change nothing.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs commit` — expect FAIL.
- [ ] **Step 3: Implement.** Normalise CRLF to LF, trim; refuse empty. `git -c core.commentChar=<c>
  commit -F - --cleanup=whitespace --trailer "<key>: <value>"…` through `write` + `checked` (hooks
  run; never `--no-verify`); then `repo::head`. Trailer keys must match `^[A-Za-z0-9-]+$` (check by
  `chars().all`, no regex crate). Hook scripts in tests use `#!/bin/sh` (git for Windows runs them
  through its `sh`).
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): commit with trailers` (`Refs: CE-22.1`).

### Task 6: Remote environment, config reads and `fetch`

**Files:**
- Create: `crates/theos-vcs/src/remote.rs` (`pub mod remote`)
- Modify: `crates/theos-vcs/src/repo.rs` (`value`)
- Test: `crates/theos-vcs/src/remote.rs` (unit), `crates/theos-vcs/tests/vcs/remote.rs`

**Interfaces:**
- Produces:
  - `pub fn repo::value(git: &Git, top: &Path, key: &str) -> Result<Option<String>, GitError>`
    (`config --get`; exit 1 is `None`);
  - `pub(crate) fn remote_env(git: &Git, top: &Path) -> Result<RemoteEnv, GitError>` and
    `RemoteEnv { pub(crate) set: Vec<(String, String)>, pub(crate) removed: Vec<String>,
    pub(crate) config: Vec<String> }`: `set` holds `GIT_TERMINAL_PROMPT=0`,
    `SSH_ASKPASS_REQUIRE=never`, and `GIT_SSH_COMMAND=ssh -o BatchMode=yes` only when
    `GIT_SSH_COMMAND`, `GIT_SSH` (process env and the runner's env) and `core.sshCommand` are all
    unset; `removed` is `["GIT_ASKPASS", "SSH_ASKPASS"]`; `config` is `["-c", "core.askPass="]`;
  - `Git::output_without(&self, args, cwd, stdin, env, removed: &[&str], cancel) -> Result<Output, GitError>`
    — `output` plus `env_remove` for each name (one spawn path: `output` calls it with `&[]`);
  - `pub fn fetch(git: &Git, top: &Path, remote: &str, cancel: &Cancel) -> Result<(), GitError>`
    (`fetch --quiet <remote>` with the write options, the remote env's `config` before the
    command, its `set` variables and its `removed` names);
  - `pub(crate) fn remote_write(git, top, command: &str, flags: &[&str], cancel) -> Result<Output, GitError>`
    — `write` with the remote env applied; Task 7's push uses it.

- [ ] **Step 1: Write the failing tests:** unit for the three `GIT_SSH_COMMAND` cases (decide from
  injected values: make the decision a pure `fn ssh_command(env_ssh_command, env_ssh, config) ->
  Option<&'static str>`); behaviour: `fetch` from a bare remote updates `refs/remotes/origin/main`
  and leaves the working tree and index untouched (`status` equal before and after); with
  `GIT_ASKPASS` set in the runner's env to a script that writes a marker, a fetch from an
  unreachable `http://127.0.0.1:9/x.git` fails and the marker never appears; a fetch cancelled
  before the start is `Cancelled`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs remote` — expect FAIL.
- [ ] **Step 3: Implement** as in the Interfaces block. The variables go per call, never through
  `Git::with_env` (§5.8).
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): fetch without prompts, credentials through the helper`.

### Task 7: `push` (CE-22.3)

**Files:**
- Create: `crates/theos-vcs/src/push.rs` (`pub mod push`), `crates/theos-vcs/src/push/porcelain.rs`
- Test: `crates/theos-vcs/src/push/porcelain.rs` (unit), `crates/theos-vcs/tests/vcs/push.rs`,
  `crates/theos-vcs/tests/vcs/credential.rs` (the HTTP server and the helper)

**Interfaces:**
- Consumes: Tasks 1, 3 (`CommitId`), 6 (`remote_write`).
- Produces:
  - `pub struct PushTarget { pub remote: String, pub branch: String, pub set_upstream: bool }`;
  - `pub enum Pushed { UpToDate, Updated { from: Option<CommitId>, to: CommitId } }`;
  - `#[non_exhaustive] pub enum PushError { Rejected { reason: Rejection, message: String },
    Failed { message: String }, Git(#[from] GitError) }`; `Display` of `Rejected` and `Failed` ends
    with `message`;
  - `#[non_exhaustive] pub enum Rejection { NotFastForward, RemoteRefused, Other }`;
  - `pub fn push(git: &Git, top: &Path, target: &PushTarget, cancel: &Cancel) -> Result<Pushed, PushError>`.

- [ ] **Step 1: Write the failing tests** (behaviour tests tagged `// AC: CE-22.3`):
  - unit: porcelain lines `"=\tHEAD:refs/heads/main\t[up to date]"`, `"*\t…\t[new branch]"`,
    `" \t…\tabc..def"`, `"!\t…\t[rejected] (non-fast-forward)"`, `"!\t…\t[rejected] (fetch first)"`,
    `"!\t…\t[remote rejected] (pre-receive hook declined)"` map to `UpToDate`, `Updated`,
    `Updated`, `NotFastForward`, `NotFastForward`, `RemoteRefused`;
  - a bare remote whose `pre-receive` prints `theos: the clue C-901 confirms a never truth` and exits
    1 → `Rejected { RemoteRefused }`, `message` contains that line (git prefixes `remote: `);
  - a remote ahead of the branch → `Rejected { NotFastForward }`, `message` is git's stderr whole;
  - a remote path that does not exist → `Failed`, `message` contains git's text;
  - a branch pushed with `set_upstream` → `branch.main.remote` and `.merge` set;
  - **credentials** (`credential.rs`): a thread serves `127.0.0.1:0` with `TcpListener`; the first
    request gets `401` with `WWW-Authenticate: Basic realm="t"`, the retry's `Authorization` header
    is recorded and answered `403`. The repository's `credential.helper` is
    `!f() { echo username=helper-user; echo password=helper-token; touch <marker>; }; f` (a shell
    function, git's documented form). `push` to `http://127.0.0.1:<port>/r.git` → `Failed`; the
    marker exists; the recorded header is `Basic base64("helper-user:helper-token")` (base64 by a
    10-line helper in the test, no crate);
  - **no helper:** the same server, no `credential.helper`, `GIT_ASKPASS` set to a script that
    writes a marker → `Failed` within 10 s (assert the elapsed time), its message is git's, the
    marker never appears.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs push credential` — expect FAIL.
- [ ] **Step 3: Implement.** `git push --porcelain [--set-upstream] <remote> HEAD:refs/heads/<branch>`
  through `remote_write` with the caller's `Cancel`.
  Exit 0: read the ref line → `UpToDate` or `Updated` (`from` from `old..new`, `None` for a new
  branch). Exit non-zero: a `!` ref line → `Rejected` with its reason; otherwise `Failed`; `message`
  is the `Output`'s stderr unchanged.
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): push with git's message whole` (`Refs: CE-22.3`).

### Task 8: `merge`, `merge_state`, `abort_merge`, `housekeeping`

**Files:**
- Create: `crates/theos-vcs/src/merge.rs` (`pub mod merge`), `crates/theos-vcs/src/housekeeping.rs`
- Test: `crates/theos-vcs/tests/vcs/merge.rs`

**Interfaces:**
- Consumes: Tasks 2, 3.
- Produces:
  - `pub enum Merged { UpToDate, FastForward { to: CommitId }, Staged, Conflicts }`;
  - `pub fn merge(git: &Git, top: &Path, upstream: &str) -> Result<Merged, GitError>`;
  - `pub fn merge_base(git: &Git, top: &Path, a: &str, b: &str) -> Result<Option<CommitId>, GitError>`;
  - `pub fn changed_since(git: &Git, top: &Path, base: &CommitId, upstream: &str) -> Result<Vec<Vec<u8>>, GitError>`
    (`diff --name-only -z --no-renames <base> <upstream>` through `read_args`);
  - `pub struct MergeState { pub head: CommitId, pub message: String }`;
  - `pub fn merge_state(git: &Git, top: &Path) -> Result<Option<MergeState>, GitError>`;
  - `pub fn conclude(git: &Git, top: &Path) -> Result<CommitId, GitError>` (`commit --no-edit`
    with the write options; hooks run);
  - `pub fn abort_merge(git: &Git, top: &Path) -> Result<(), GitError>`;
  - `pub fn housekeeping(git: &Git, top: &Path, cancel: &Cancel) -> Result<(), GitError>`.

- [ ] **Step 1: Write the failing tests:** a clone behind its remote → `FastForward` with the
  remote's head; diverged without overlap → `Staged`, `merge_state` gives `MERGE_HEAD` and a
  message starting `Merge`, `conclude` makes a commit with two parents; diverged with the same line
  changed → `Conflicts`, `ls-files -u` non-empty; `abort_merge` after `Staged` and after `Conflicts`
  restores `HEAD` and removes `MERGE_HEAD`; with user settings `merge.ff=false`,
  `merge.autoStash=true` a behind clone still gives `FastForward` and no stash entry exists;
  `changed_since` lists the upstream's paths; `housekeeping` on a fresh repo is `Ok`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs merge` — expect FAIL.
- [ ] **Step 3: Implement.** `merge --ff --no-commit --no-edit --no-autostash --no-squash
  <upstream>` through `write` (exit code read, not `checked`): exit 0 and `HEAD` moved with no
  `MERGE_HEAD` → `FastForward`; exit 0 with `MERGE_HEAD` → `Staged`; exit 0 and `HEAD` unchanged
  → `UpToDate`; exit 1 with `MERGE_HEAD` and unmerged entries (`ls-files -u -z` non-empty) →
  `Conflicts`; anything else → `checked`'s error. Never parse the message. `merge_state` reads
  `rev-parse -q --verify MERGE_HEAD` and the file at `rev-parse --git-path MERGE_MSG`.
  `housekeeping` runs `-c gc.autoDetach=false -c maintenance.autoDetach=false maintenance run
  --auto` with the caller's `Cancel` (not through `write_args`, whose `gc.auto=0` would make it a
  no-op).
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): merge an upstream, its state, abort, housekeeping`.

### Task 9: Studio: the repo lock outside commands, stage and unstage by record

**Files:**
- Create: `crates/theos-studio/src/git_write/mod.rs` (`pub mod git_write` in `lib.rs`),
  `crates/theos-studio/src/git_write/stage.rs`
- Modify: `crates/theos-studio/src/command/pipeline.rs` (move `take_lock` to a shared place),
  `crates/theos-studio/src/workspace.rs`, `crates/theos-studio/src/error.rs`
- Test: `crates/theos-studio/tests/studio/git_write.rs` (new, registered in `tests/studio/main.rs`)

**Interfaces:**
- Consumes: Task 3; P46's `changes`, `RecordChange`, `status::Entry`.
- Produces:
  - `pub(crate) fn Workspace::hold(&mut self, purpose: &str) -> Result<Holding<'_>, LockError>` —
    the pipeline's `Held::take` logic (reentry gives `Reentrant` at once), returning a guard that
    clears `commands.locked` and releases on drop; the pipeline uses it too;
  - `StudioError::Conflicted { id: String }`, `StudioError::NotDiscardable { path: String, why: String }`,
    `StudioError::Lock(#[from] LockError)` (if not present);
  - `pub fn stage_records(workspace: &mut Workspace, git: &Git, records: &[&RecordChange], files: &[&Entry]) -> Result<(), StudioError>`
    and `unstage_records` with the same signature (purposes `stage`, `unstage`).

- [ ] **Step 1: Write the failing tests** (on `Repo::committed("")` from `tests/studio/repo.rs`
  and the `staged` fixture project):
  - a record renamed on disk with P06's `Rename` → `stage_records` of its `RecordChange` gives one
    staged rename in `changes`; `unstage_records` puts it back;
  - a conflicted record → `Conflicted { id }`, nothing staged;
  - a project at `content/` in the repository with a changed `README` at the top → `stage_records`
    with that `Entry` from `Changes.files` stages it (Review Focus 2);
  - `staging_uses_git_spelling` (Review Focus 5): a record committed as `lore/truths/T-900.md` and
    edited on disk → the `RecordChange.repo_path` is staged (on Windows and macOS also with the
    folder spelled `Lore` on disk: `cfg(any(windows, target_os = "macos"))`, reason in a comment);
  - the lock held by a helper process (`spawn_helper`, as `tests/studio/lock.rs` does) → stage
    returns `Lock(Busy)` after the workspace's lock wait (set it short for the test), nothing staged;
  - a workspace already inside `hold` that calls `stage_records` → `Lock(Reentrant)` at once.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio git_write` — expect FAIL.
- [ ] **Step 3: Implement.** Collect `repo_path` and `repo_from` of each record and the path(s) of
  each `Entry`; refuse any `conflict` first; one `theos_vcs::stage::stage` / `unstage` call under
  `hold`.
- [ ] **Step 4: Run** `cargo nextest run -p theos-studio` — expect PASS (the pipeline's lock tests
  still pass).
- [ ] **Step 5: Commit** `feat(studio): stage and unstage by record under the repo lock`.

### Task 10: `Discard`, undoable

**Files:**
- Create: `crates/theos-studio/src/command/discard.rs`
- Modify: `crates/theos-studio/src/command/mod.rs` (`Command::restores`, `pub use discard::Discard`),
  `crates/theos-studio/src/command/pipeline.rs` (verdict skip)
- Test: `crates/theos-studio/src/command/discard.rs` (unit), `crates/theos-studio/tests/studio/discard.rs`

**Interfaces:**
- Consumes: Tasks 4, 9; P06's `Plan`, `FileChange`, `Base`, `read_file`, `project_path`.
- Produces:
  - `Command::restores(&self) -> bool { false }` (default) — when `true`, the pipeline skips
    step 5's blocking (it still computes `findings` and `introduced`) and keeps every other step
    (paths, boundary, bases);
  - `pub struct Discard` with
    `Discard::record(workspace: &Workspace, git: &Git, record: &RecordChange) -> Result<Discard, StudioError>`
    and `Discard::file(workspace: &Workspace, git: &Git, entry: &Entry) -> Result<Discard, StudioError>`;
    label `discard <path>`; `restores()` is `true`; its plan writes exactly the bytes read at
    construction, never normalised.

- [ ] **Step 1: Write the failing tests:**
  - modified record → file holds the index's bytes; `undo(Scope::Newest)` restores the edit
    byte for byte; `redo` discards again;
  - with `core.autocrlf=true` the discarded file holds CRLF (checkout bytes) and `changes` shows
    the record unchanged;
  - unstaged deletion → recreated; paired unstaged rename → old path recreated, new path removed,
    undo brings back the rename; untracked record → removed, undo recreates it;
  - only staged changes → empty plan (`Outcome.changed` empty);
  - an index version with a dangling link while the working copy fixed it → the discard goes
    through and `Outcome.introduced` names the error;
  - edited again after `Discard::record` was built → `CommandError::Stale`;
  - conflicted → `Conflicted`; a path outside the project (Review Focus 2) or a `.gitignore` →
    `NotDiscardable` whose message names `git restore`; a link in the index → `NotDiscardable`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio discard` — expect FAIL.
- [ ] **Step 3: Implement.** At construction: map repo paths to project paths with
  `repo_path::project_path` (outside → `NotDiscardable`); read the target with
  `checkout_bytes` (`Absent` → delete, `File` → write, `NotAFile` → `NotDiscardable`); read the
  working bytes now for the `before`. Run P06's boundary rules on the planned changes at
  construction too, so `NotDiscardable` comes before the pipeline. `plan` re-reads the working
  files and fails with `Stale` when they differ from the construction-time `before`.
- [ ] **Step 4: Run** `cargo nextest run -p theos-studio` — expect PASS (every existing command's
  verdict unchanged: `restores()` defaults to `false`).
- [ ] **Step 5: Commit** `feat(studio): discard a record's unstaged changes, undoable`.

### Task 11: The checked commit (CE-22.1)

**Files:**
- Create: `crates/theos-studio/src/git_write/commit.rs`
- Test: `crates/theos-studio/tests/studio/commit.rs`

**Interfaces:**
- Consumes: Tasks 5, 9; P05's `check_staged`, `StagedCheck`.
- Produces:
  - `pub struct Committed { pub id: CommitId, pub check: StagedCheck }`;
  - `#[non_exhaustive] pub enum CommitError { Blocked(StagedCheck), NothingStaged, Message(String),
    Conflicted(Vec<String>), Lock(LockError), Git(GitError), Studio(StudioError) }` (`thiserror`,
    sources kept; `Message` covers empty messages, a bad trailer and no free comment character);
  - `pub fn commit(workspace: &mut Workspace, git: &Git, message: &str, trailers: &[Trailer]) -> Result<Committed, CommitError>`.

- [ ] **Step 1: Write the failing tests**, each `// AC: CE-22.1`, on `Repo::committed("")` with the
  `staged` fixture (`variant("C-901.md")` is the never-confirm error; the fixture's standing
  `lore/three-clue` on T-900 is the warning):
  - `a_staged_error_blocks_the_commit`: stage C-901 → `Blocked(check)` with
    `lore/never-confirm` on C-901; `repo::head` unchanged; C-901 still staged;
  - `warnings_do_not_block_the_commit`: stage a clean edit of T-900 → `Committed`; `head` is the new
    commit with the message and trailer; `check.warnings() > 0`, `check.errors() == 0`;
  - `an_error_only_in_the_working_tree_is_allowed`;
  - `a_staged_error_fixed_only_on_disk_still_blocks`;
  - `a_staged_deletion_that_breaks_a_link_blocks` (delete T-900 with `git rm`, C-900 cites it);
  - nothing staged → `NothingStaged`; empty message → `Message`; a conflict → `Conflicted`;
  - the lock held by a helper → `Lock(Busy)`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio commit` — expect FAIL.
- [ ] **Step 3: Implement** spec §5.5 steps 1–8 in order: `hold("commit")` → message check →
  `staged::read` (no change and no `merge_state` → `NothingStaged`; unmerged entries →
  `Conflicted`) → `check_staged` (`blocks()` → `Blocked`) → `theos_vcs::commit::commit` → return
  with the check. No index transaction is open when git starts (`check_staged` returns first).
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(studio): commit after the staged check` (`Refs: CE-22.1`).

### Task 12: Pull and abort

**Files:**
- Create: `crates/theos-studio/src/git_write/pull.rs`
- Test: `crates/theos-studio/tests/studio/pull.rs`

**Interfaces:**
- Consumes: Tasks 6, 8, 11 (the check-then-conclude path), 9.
- Produces:
  - `pub enum Pulled { UpToDate, FastForward { to: CommitId }, Merged { id: CommitId, check: StagedCheck },
    Conflicts(Changes), Refused { check: StagedCheck, changes: Changes } }`;
  - `#[non_exhaustive] pub enum PullError { Detached, NoUpstream { branch: String },
    LocalChanges { records: Vec<RecordChange>, files: Vec<Entry> }, MergeInProgress, Lock(LockError),
    Git(GitError), Studio(StudioError) }`;
  - `pub fn pull(workspace: &mut Workspace, git: &Git, cancel: &Cancel) -> Result<Pulled, PullError>`;
  - `pub fn abort_merge(workspace: &mut Workspace, git: &Git) -> Result<(), StudioError>`;
  - `pub(crate) fn upstream(git: &Git, top: &Path, branch: &str) -> Result<Option<(String, String)>, GitError>`
    — `(remote, branch)` from `branch.<name>.remote` and `branch.<name>.merge` (strip
    `refs/heads/`) via `repo::value`; Task 13 uses it.

- [ ] **Step 1: Write the failing tests** (a bare remote and two clones of the `staged` fixture):
  - behind → `FastForward`, the index synced (a query sees the pulled record);
  - diverged, clean, merged result without errors → `Merged` with two parents and the warning;
  - diverged where the upstream adds C-901 (never-confirm) → `Refused` with the error;
    `merge_state` is `Some`; fixing C-901, staging and Task 11's `commit` concludes with two parents;
  - diverged with a conflicting edit of T-900 → `Conflicts` whose `RecordChange` for T-900 has a
    `conflict`; `abort_merge` restores `HEAD`;
  - an unstaged edit of the record the upstream changed → `LocalChanges` naming it; nothing merged;
  - diverged with an unrelated staged file → `LocalChanges` (builder probe);
  - an unrelated unstaged edit on a fast-forward → `FastForward`, the edit kept;
  - detached → `Detached`; no upstream → `NoUpstream`; a merge in progress → `MergeInProgress`;
  - `a_gone_upstream_is_no_upstream` (Review Focus 4): delete the remote branch and fetch with
    `--prune` → `NoUpstream { branch: "main" }`;
  - a fetch cancelled before the start → `Git(Cancelled)` and no lock was taken (a helper holds
    the lock during the call and the call still returns at once).
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio pull` — expect FAIL.
- [ ] **Step 3: Implement** spec §5.6 steps 1–6. Fetch outside `hold`; inside `hold("pull")`:
  status again, the local-changes check (`changed_since` against every changed path under the
  repository's `Folding`, plus every staged path when ahead > 0 and behind > 0), `merge`; on
  `Staged`: `check_staged` → errors: `Refused` with `changes`; else `conclude` → `Merged`. After
  releasing the lock, `workspace.index_mut().sync()`.
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(studio): pull by fetch and a checked merge`.

### Task 13: Push by record's branch (CE-22.3, studio part)

**Files:**
- Create: `crates/theos-studio/src/git_write/push.rs`
- Test: `crates/theos-studio/tests/studio/push.rs`

**Interfaces:**
- Consumes: Tasks 7, 12 (`upstream`).
- Produces:
  - `#[non_exhaustive] pub enum PushFailed { Detached, NoUpstream { remotes: Vec<String> },
    Push(#[from] PushError), Studio(StudioError) }`, `Display` of `Push` is the `PushError`'s;
  - `pub fn push(workspace: &mut Workspace, git: &Git, cancel: &Cancel) -> Result<Pushed, PushFailed>`;
  - `pub(crate) fn push_target(git, top, branch) -> Result<PushTarget, PushFailed>` — remote:
    `branch.<b>.pushRemote` → `remote.pushDefault` → `branch.<b>.remote`; branch: the upstream
    branch when the push remote is the upstream's remote, else `<b>`; none of the three → the only
    remote (`git remote`) with `set_upstream`, else `NoUpstream { remotes }`.

- [ ] **Step 1: Write the failing tests** (`// AC: CE-22.3` on the message tests):
  - a remote whose `pre-receive` refuses → `Push(Rejected { RemoteRefused, message })`; the
    `Display` of the error contains the hook's line;
  - `branch.main.pushRemote=fork` with remotes `origin` and `fork` → pushed to `fork` under `main`,
    `origin` untouched; `remote.pushDefault=fork` the same;
  - an upstream `origin/trunk` for local `main` → pushed to `trunk`;
  - no upstream, one remote → pushed with upstream set; two remotes → `NoUpstream { ["fork",
    "origin"] }`; detached → `Detached`;
  - the lock held by a helper → push still succeeds (no lock).
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio push` — expect FAIL.
- [ ] **Step 3: Implement** as in the Interfaces block; no `hold`.
- [ ] **Step 4: Run** — expect PASS.
- [ ] **Step 5: Commit** `feat(studio): push to git's push remote` (`Refs: CE-22.3`).

### Task 14: Hostile settings, docs, README

**Files:**
- Test: `crates/theos-studio/tests/studio/git_settings.rs`
- Modify: `crates/theos-vcs/README.md` ("What P47 provides" table, "Who uses what": P47 row
  rewritten, P07 row gains worktrees and the approval merge, P13 row gains the writes), the crate
  docs of `git_write` and of each new vcs module

- [ ] **Step 1: Write the test:** in one repository set `pull.rebase=true`, `pull.ff=only`,
  `merge.ff=false`, `merge.autoStash=true`, `push.default=matching`, `commit.verbose=true`,
  `core.commentChar=;`, `color.ui=always`, `core.quotePath=true`, `gc.auto=1`; stage by record,
  commit, pull a fast-forward and a diverged upstream, push; every result equals the run without
  them (`Committed`, `FastForward`, `Merged`, `Pushed::Updated`), and `git stash list` is empty.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio git_settings` — fix any setting that leaks
  in the module that runs that git command (not in the test).
- [ ] **Step 3: Write the README and crate docs.**
- [ ] **Step 4: Run** `cargo xtask gate` and `cargo xtask ac-trace --phase P47` — expect every
  check `pass` and CE-22.1 and CE-22.3 each with passing tests.
- [ ] **Step 5: Commit** `docs(vcs): what P47 provides`.

## After the tasks

- Final review loop (CLAUDE.md), `cargo xtask gate` on Windows, then `docs/status.md` → `pr-open`
  and the PR `feat(vcs): git write (P47)` with labels `windows` and `macos` (Task 1 changes the
  wait shared by both process-tree paths; spec §5.12).
- Not in this phase: the Changes screen (P13), worktrees and approval merges (P07).
