# P46 · Git read — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (CLAUDE.md) to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `theos-vcs` checks git's version and reads status, history and files at any version, with
cancellable runs that stop git's whole process tree; `theos-content` diffs records by field and by
word; `theos-studio` shows changes, diffs and history by record.

**Architecture:** three layers, each usable alone.
- `theos-vcs` (bytes, git spelling, no record knowledge): `GitVersion`, `Cancel`,
  `run_cancellable`, one module of read options, `read`, `status`, `history`.
- `theos_content::diff` (texts only, never git): `words`, `fields`, `record`.
- `theos-studio` (records): `changes`, `diff`, `history`, on P05's path mapping moved to a shared
  module.

**Tech stack:** Rust 2024 (toolchain 1.99.1). git ≥ 2.38 on the PATH (tests use the real git).
Crates:
- already in the workspace: `thiserror`, `tracing`, `tempfile`, `which`, `similar`,
  `unicode-normalization`, `proptest` (dev);
- new: `win32job` 2.0.3 (Windows only) and `nix` 0.31 with the `signal` feature (Unix only;
  already in `Cargo.lock` as a dependency of a dependency).

**Spec:** `docs/specs/phases/P46-git-read.md` (approved in #52), with its change in PR #58
(process trees through `win32job` and a process group; Jann's choice of 27 Nov 2026). Read both
with this plan. Interfaces from P05: `docs/specs/phases/P05-cli-and-hook.md` §5.2, §6.

**Order:** Task 4 starts only once #58 is merged; Tasks 6, 7 and 11–13 need Task 4's `Cancel`.
If #58 is not merged when Task 3 is done, build Tasks 9 and 10 (no git) next, then Task 5, and
wait for #58 before Task 4.

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
The phase's acceptance criteria, verbatim (umbrella §3 P46):

**CE-21 Git read**
- CE-21.1 A renamed record keeps its history.
- CE-21.2 Field diff shows old and new per field; prose diff is word-level.
- CE-21.3 A missing or too-old git gives a clear message.

From the spec, binding on every task:
- The minimum git is `GitVersion::MINIMUM` = 2.38.0; no other copy of the number (§5.2).
- Every git read passes `--no-optional-locks` and builds its arguments from the one list of
  read options (§5.3). Paths stay bytes in git's spelling inside `theos-vcs`.
- `theos-vcs` depends on no other workspace crate. `theos_content::diff` never runs git.
- Nothing in this phase takes P06's repo lock or writes P03's cache.
- Engineering directives 1–4 of `CLAUDE.md` bind every implementer; put them in every brief.
- Tests carry `// AC: <id>` when they prove an AC. Test git repositories are isolated as in P05
  (`GIT_CONFIG_GLOBAL` on an empty file, `GIT_CONFIG_NOSYSTEM=1`, identities by `-c`).

## Review Focus

The inputs the spec implies that are most likely to break a user, with the task that pins each:
1. A path with a space, `[`, `]`, `:`, a leading `-` and non-ASCII text, in status, history and
   read: git must treat it as a literal path, never a pattern or an option (Tasks 5, 6, 7).
2. More untracked files than fit on a Windows command line (32 767 characters) during status:
   the temporary index takes them on stdin (Task 6).
3. A user's git settings that change git's output (`color.ui=always`, `status.renames=false`,
   `log.showSignature=true`, …): the reads give the same answers (Task 8).
4. A record with CRLF line endings on one side of a diff: no change is shown, and spans point
   into the LF texts the diff returns (Tasks 9, 10).
5. A cancel while git writes more than a pipe holds, and a cancel before the start: no process
   of the tree survives, no temporary index is left behind (Tasks 4, 6).

---

### Task 1: `GitVersion`

**Files:**
- Create: `crates/theos-vcs/src/version.rs`
- Modify: `crates/theos-vcs/src/lib.rs` (module and `pub use version::GitVersion`)

**Interfaces:**
- Produces: `pub struct GitVersion { pub major: u32, pub minor: u32, pub patch: u32 }` with
  `Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord` and `Display` as `2.38.0`;
  `pub const MINIMUM: GitVersion` (2.38.0); `pub fn parse(answer: &str) -> Option<GitVersion>`.

- [ ] **Step 1: Write the failing unit tests** in `version.rs`, each with `// AC: CE-21.3`:
  - `parse("git version 2.39.3 (Apple Git-146)\n") == Some(2.39.3)`;
  - `"git version 2.56.1.windows.1"` → 2.56.1; `"git version 2.38.0.rc1"` → 2.38.0;
    `"git version 2.44.0.123.gabcdef"` → 2.44.0; `"git version 2.40"` → 2.40.0;
  - refused (`None`): `""`, `"git version"`, `"git version abc"`, `"hg version 2.40.0"`,
    `"git version .2.3"`, a number above `u32::MAX`;
  - order: 2.37.99 < `MINIMUM`, 2.38.0 == `MINIMUM`, `parse("git version 2.38.0.rc1")` ≥ `MINIMUM`;
  - `MINIMUM.to_string() == "2.38.0"`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs version` — expect FAIL (no module).
- [ ] **Step 3: Implement** `GitVersion` (spec §5.2): strip `git version `, take at most three
  dot-separated runs of ASCII digits from the start, stop at the first other character; a
  missing minor is refused, a missing patch is 0. No crate parses git's version (spec §4.3).
- [ ] **Step 4: Run** the tests — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): read git's version` (`Refs: CE-21.3`, `Phase: P46`).

### Task 2: The version check in `Git::find`, and the new errors

**Files:**
- Modify: `crates/theos-vcs/src/git.rs` (`Git` gets a `version` field; `find`, `find_in`,
  `GitError`)
- Test: `crates/theos-vcs/src/git.rs` (unit), `crates/theos-vcs/tests/vcs/git.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces:
  - `GitError::TooOld { program: PathBuf, found: GitVersion, required: GitVersion }` and
    `GitError::Cancelled { args: Vec<String> }`;
  - `Git::version(&self) -> GitVersion`;
  - a private `fn checked(program: PathBuf, answer: &[u8]) -> Result<GitVersion, GitError>`
    that `find` uses after running `git --version`, so the unit tests reach the check without a
    fake program.

- [ ] **Step 1: Write the failing tests**, each with `// AC: CE-21.3`:
  - unit: `checked(p, b"git version 2.37.9\n")` is `TooOld { found: 2.37.9, required: MINIMUM,
    program: p }`; `checked(p, b"git version 2.38.0\n")` is `Ok(2.38.0)`; `checked(p, b"nonsense")`
    is `GitError::Output` whose message holds `nonsense`;
  - unit: the `Display` of `NotFound` is exactly "git was not found on the PATH. theos needs git
    2.38 or newer: install it from https://git-scm.com/downloads, then open a new terminal or
    restart the editor.";
  - unit: the `Display` of `TooOld { program: "/usr/bin/git", found: 2.34.1, required: MINIMUM }`
    is exactly "git 2.34.1 (/usr/bin/git) is too old. theos needs git 2.38 or newer: update it
    from https://git-scm.com/downloads." (the path shown with `Path::display`);
  - unit: `Cancelled { args: ["log", "--follow"] }` displays "git log --follow was cancelled";
  - behaviour (`tests/vcs/git.rs`): `Sandbox::git()?.version() >= GitVersion::MINIMUM`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs` — expect the new tests to FAIL.
- [ ] **Step 3: Implement.** `find` = search the PATH (as today) → resolve the environment (as
  today) → run `git --version` in the current folder with the resolved environment → `checked`.
  Both messages build the minimum from `GitVersion::MINIMUM`, never a literal `2.38`. Keep
  `#[non_exhaustive]`.
- [ ] **Step 4: Run** `cargo nextest run -p theos-vcs -p theos-studio -p theos` — expect PASS
  (every existing caller of `Git::find` now gets the check).
- [ ] **Step 5: Commit** `feat(vcs): refuse a missing or too-old git with a clear message`
  (`Refs: CE-21.3`).

### Task 3: CE-21.3 through the `theos` binary

**Files:**
- Create: `crates/theos/tests/cli/git_version.rs`; register it in `crates/theos/tests/cli/main.rs`
- Modify: `crates/theos/tests/cli/common.rs` (one helper, below)

**Interfaces:**
- Consumes: Task 2's messages.
- Produces: `Sandbox::run_with_path(&self, args: &[&str], cwd: &Path, path: &OsStr) ->
  io::Result<Output>` — like `run`, with `PATH` replaced for the child.

**The stand-in git** (spec §5.10, left to the plan): a folder in the sandbox holding one program
named `git` that prints `git version 2.37.9` and exits 0:
- Unix (`#[cfg(unix)]`): a file `git` with `#!/bin/sh\necho "git version 2.37.9"\n`, mode 0755.
- Windows (`#[cfg(windows)]`): a file `git.bat` with `@echo git version 2.37.9\r\n`; `which`
  finds it through `PATHEXT`, which the child inherits.

- [ ] **Step 1: Write the failing tests** (`// AC: CE-21.3` on each), in a copy of the P05
  fixture project `staged` inside a git repository made by the sandbox (as the existing
  `check.rs` tests do):
  - `no_git_on_the_path_is_explained`: `run_with_path(["check", "--staged"], project, <an empty
    folder>)` exits 2 and its stderr holds "git was not found on the PATH" and "git 2.38 or
    newer";
  - `a_too_old_git_is_explained_on_unix` (`#[cfg(unix)]`) and `…_on_windows`
    (`#[cfg(windows)]`): with the stand-in folder as `PATH`, exit 2, stderr holds
    "git 2.37.9 (", the stand-in's path, "is too old" and "git 2.38 or newer".
- [ ] **Step 2: Run** `cargo nextest run -p theos git_version` — Task 2 already provides the
  behaviour, so expect PASS; first check each test fails when its expected text is altered (a
  red run with a wrong expectation, then restore). If `theos` prints the messages twice or with
  a prefix that hides them, fix `crates/theos/src` printing, not the test.
- [ ] **Step 3: Commit** `test(cli): explain a missing or too-old git` (`Refs: CE-21.3`). The
  PR gets the labels `windows` and `macos` (per-OS tests).

### Task 4: `Cancel`, `run_cancellable` and process trees (after spec PR #58 is merged)

**Files:**
- Create: `crates/theos-vcs/src/cancel.rs`, `crates/theos-vcs/src/tree.rs` (the per-OS code:
  `#[cfg(windows)] mod windows; #[cfg(unix)] mod unix;` inside, or two files
  `tree/windows.rs`, `tree/unix.rs`)
- Modify: `crates/theos-vcs/src/git.rs` (`run` calls `run_cancellable`), `crates/theos-vcs/Cargo.toml`,
  root `Cargo.toml` (`[workspace.dependencies]`: `win32job = "=2.0.3"`, `nix = { version = "0.31",
  default-features = false, features = ["signal"] }`), `crates/theos-vcs/src/lib.rs`
- Test: `crates/theos-vcs/tests/vcs/cancel.rs`

**Interfaces:**
- Produces:
  - `#[derive(Clone, Debug, Default)] pub struct Cancel(Arc<AtomicBool>)` with `new`,
    `cancel(&self)`, `is_cancelled(&self) -> bool`;
  - `Git::run_cancellable(&self, args: &[&OsStr], cwd: &Path, stdin: Option<&[u8]>,
    env: &[(&str, &str)], cancel: &Cancel) -> Result<Vec<u8>, GitError>`;
  - `Git::run` unchanged in signature, now `self.run_cancellable(…, &Cancel::new())`.
- Dependencies in `crates/theos-vcs/Cargo.toml`: `win32job` under
  `[target.'cfg(windows)'.dependencies]`, `nix` under `[target.'cfg(unix)'.dependencies]`,
  `tracing.workspace = true`.

- [ ] **Step 1: Write the failing tests** in `tests/vcs/cancel.rs`:
  - `a_cancel_stops_the_whole_tree`: git runs `-c alias.slow='!sleep 2 && echo done > <marker>'
    slow` (marker path with `/` separators) through `run_cancellable` on a thread; the test
    cancels 300 ms after the start; expect `Err(GitError::Cancelled { .. })` within 300 + 50 ms of
    the cancel, then sleep 3 s and assert the marker does not exist. One test body, run on every
    OS (the per-OS code is inside `tree`); name the OS in the failure message;
  - `a_cancel_before_the_start_starts_nothing`: a cancelled flag and the same alias → `Cancelled`
    at once, marker never written;
  - `a_large_answer_is_read_while_cancel_is_watched`: `git cat-file --batch` with 4 MiB on stdin
    (one blob written with `hash-object -w` and asked for 64 times) returns all bytes;
  - `run_still_reports_failures`: `run(["rev-parse", "--verify", "nope"])` is `Failed` with
    `code: Some(128)` and git's stderr.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs cancel` — expect FAIL.
- [ ] **Step 3: Implement** (spec §5.3 as changed by #58):
  - spawn with `std::process::Command` (stdin, stdout, stderr as today);
  - Windows: before `spawn`, `win32job::Job::create()` with `limit_kill_on_job_close()` set
    through `query_extended_limit_info`/`set_extended_limit_info`; right after `spawn`,
    `job.assign_process(child.as_raw_handle() as isize)`; a failed assignment kills git and is
    `GitError::Io`; on cancel, drop the job (kill-on-close ends every process in it);
  - Unix: `CommandExt::process_group(0)` before `spawn`; on cancel
    `nix::sys::signal::killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL)`;
  - then `wait` and return `Cancelled { args }`;
  - the wait: reader threads for stdout and stderr and a writer thread for stdin, as in P05; the
    calling thread polls `try_wait` starting at 1 ms and doubling to at most 20 ms, checking the
    flag each time. (Probe, 27 Nov: `try_wait` costs about 3 µs.)
  - log each command at `debug` (`args`, `cwd`, duration) and failures at `warn`.
- [ ] **Step 4: Run** `cargo nextest run -p theos-vcs -p theos-studio -p theos` — expect PASS.
  Time `cargo nextest run -p theos-vcs` before and after; a slowdown above 5 % is a defect to
  report, not to accept.
- [ ] **Step 5: Commit** `feat(vcs): cancel git and stop its whole process tree`.

### Task 5: Read options and `read` (a file at a version)

**Files:**
- Create: `crates/theos-vcs/src/read_args.rs` (the one list), `crates/theos-vcs/src/read.rs`
- Modify: `crates/theos-vcs/src/lib.rs`; `crates/theos-vcs/src/staged.rs` (its `--no-optional-locks`
  reads use the list)
- Test: `crates/theos-vcs/tests/vcs/read.rs`

**Interfaces:**
- Produces:
  - `pub(crate) fn read_args<'a>(command: &'a str, flags: &[&'a str]) -> Vec<&'a OsStr>` (or an
    equivalent builder) that puts, in this order: `--no-optional-locks`, `--literal-pathspecs`,
    `-c color.ui=never`, `-c log.showSignature=false`, `-c core.quotePath=false`,
    `-c i18n.logOutputEncoding=UTF-8`, then `command`, then `flags`. Every read of Tasks 5–7 and
    P05's staged reads go through it;
  - `pub enum Version<'a> { Head, Index, Rev(&'a str), WorkTree }`;
  - `pub enum Content { Absent, File(Vec<u8>), NotAFile }`;
  - `pub fn read(git: &Git, top: &Path, version: Version<'_>, path: &[u8]) -> Result<Content, GitError>`.

- [ ] **Step 1: Write the failing tests** (`tests/vcs/read.rs`):
  - HEAD, index and working tree of one file that differs in all three give the three texts;
  - `Rev("<commit id>")` and `Rev("main")` give that commit's bytes;
  - a path absent from a version is `Absent`; before the first commit, `Head` is `Absent`;
  - an intent-to-add entry (`git add -N`) reads `Absent` from `Index`;
  - a path named `1:notes.md` reads from the index as itself (uses `:0:<path>`); a path named
    `-x.md`, one with a space and `[x]`, and one with `é` read correctly;
  - a symbolic link (`#[cfg(unix)]`, made with `std::os::unix::fs::symlink`) and a folder are
    `NotAFile` in `WorkTree`; a committed link is `NotAFile` in `Head`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-vcs read` — expect FAIL.
- [ ] **Step 3: Implement** (spec §5.4): `rev-parse --verify --quiet --end-of-options
  <rev>:<path>` (`:0:<path>` for the index; exit code 1 with empty stderr is `Absent`); the mode
  from `ls-tree -z <rev> -- <path>` (`ls-files -s -z -- <path>` for the index), reusing P05's
  `is_regular`; the bytes through P05's `blobs`; intent-to-add as P05 finds it; `WorkTree` with
  `std::fs::symlink_metadata` then `std::fs::read`.
- [ ] **Step 4: Run** `cargo nextest run -p theos-vcs -p theos-studio` — expect PASS.
- [ ] **Step 5: Commit** `feat(vcs): read a file at any version`.

### Task 6: `status`

**Files:**
- Create: `crates/theos-vcs/src/status.rs`, `crates/theos-vcs/src/status/parse.rs`
- Test: `crates/theos-vcs/src/status/parse.rs` (unit), `crates/theos-vcs/tests/vcs/status.rs`

**Interfaces:**
- Consumes: Task 5's `read_args`; Task 4's `Cancel` and `run_cancellable`.
- Produces exactly the types of spec §5.5: `Status`, `Branch { head, commit, upstream }`,
  `Upstream { name, ahead, behind }`, `Entry::{Changed, Renamed, Conflicted, Untracked}`,
  `Change::{Unchanged, Modified, TypeChanged, Added, Deleted, Renamed, Copied}`,
  `Conflict::{BothDeleted, AddedByUs, DeletedByThem, AddedByThem, DeletedByUs, BothAdded,
  BothModified}` (DD AU UD UA DU AA UU), and
  `pub fn status(git: &Git, top: &Path, cancel: &Cancel) -> Result<Status, GitError>`.

- [ ] **Step 1: Write the failing unit tests** of `parse.rs` on literal `-z` porcelain v2 bytes:
  the `# branch.*` headers (with and without upstream, `(initial)`, `(detached)`), `1`, `2`
  (rename and copy, in the index and in the working tree), `u` and `?` lines; a path holding a
  space and non-ASCII bytes; a truncated line is `GitError::Output`.
- [ ] **Step 2: Write the failing behaviour tests** in `tests/vcs/status.rs`:
  - an unstaged rename with `id:` rewritten (the probe of spec §4.2) is one
    `Renamed { from: "mother-ilse.md", to: "mother-ilsa.md", staged: Unchanged, unstaged: Renamed, .. }`;
  - the real index file's bytes are identical before and after `status`;
  - a staged rename with an unstaged edit; a file staged then deleted on disk; a user's own
    `git add -N` file is `Changed { unstaged: Added }`; untracked files in an untracked folder are
    listed one by one; ignored files are not listed;
  - no index file (fresh `git init` with a file) and no commit yet; a detached HEAD
    (`head: None`); an upstream ahead 1 and behind 1 (a second clone in the sandbox);
  - a conflict (`UU`) after a failed `git merge`;
  - 3 000 untracked files whose paths total more than 40 000 characters are all `Untracked`
    (Review Focus 2);
  - a path with a space, `[`, `]` and `é` (Review Focus 1);
  - a `Cancel` set before the call returns `Cancelled` and leaves no temporary index;
  - inside a hook's index (`GIT_INDEX_FILE` in the runner's environment pointing at a copy) the
    copy is what `status` reads.
- [ ] **Step 2b: Write the failing unit tests** in `status.rs` of the seam
  `pub(crate) fn status_in(git: &Git, top: &Path, cancel: &Cancel, temp: &Path) -> Result<Status,
  GitError>` (`status` calls it with `std::env::temp_dir()`): with a fresh temp folder as `temp`,
  the folder is empty after a success, after an error (a `top` that is not a repository) and
  after a cancel.
- [ ] **Step 3: Run** `cargo nextest run -p theos-vcs status` — expect FAIL.
- [ ] **Step 4: Implement** the five steps of spec §5.5 in order. The temporary index is a
  `tempfile::NamedTempFile` (deleted on drop, so on every return path); `add` gets
  `--intent-to-add --pathspec-from-file=- --pathspec-file-nul` with the NUL-separated paths on
  stdin and the read options' `--literal-pathspecs`. Skip steps 3 and 5 when nothing is
  untracked (ruling 7).
- [ ] **Step 5: Run** the tests — expect PASS. **Commit** `feat(vcs): read status with unstaged
  renames paired`.

### Task 7: `history`

**Files:**
- Create: `crates/theos-vcs/src/history.rs`, `crates/theos-vcs/src/history/parse.rs`
- Test: unit tests in `parse.rs`; `crates/theos-vcs/tests/vcs/history.rs`

**Interfaces:**
- Consumes: Task 5's `read_args`, Task 6's `Change`, Task 4's `Cancel`.
- Produces the types of spec §5.6: `Commit { id, author: Person, time: i64, offset_minutes: i32,
  subject, trailers: Vec<Trailer>, change: Change, path: Vec<u8>, from: Option<Vec<u8>> }`,
  `Person { name, email }`, `Trailer { key, value }`, `Page { skip, max }`, and
  `pub fn history(git: &Git, top: &Path, path: &[u8], page: Page, cancel: &Cancel)
  -> Result<Vec<Commit>, GitError>`.

The format (decided here, spec §10): each commit starts with the byte `0x1e`; its fields are
separated by `0x1f`: `%H`, `%aN`, `%aE`, `%ad` (`--date=raw`: `<seconds> <±hhmm>`), `%s`,
`%(trailers:unfold,separator=%x1d,key_value_separator=%x1c)`. With `-z`, git ends the format
with a NUL, and the name-status lines follow as NUL-separated fields (`R083`, old, new). A field
that holds a separator byte is impossible in git's output for these placeholders except `%s` and
trailer values; the parser treats the record framing as authoritative and fails with
`GitError::Output` on a commit with the wrong number of fields rather than guessing.

- [ ] **Step 1: Write the failing unit tests** of the parser on literal bytes: one commit with a
  rename and a trailer; three commits; a subject with `%`, `"`, a tab and `é`; two trailers, one
  folded over two lines in the message (unfolded by git); a commit whose timezone is `-0330`
  (`offset_minutes == -210`); an empty answer (empty `Vec`).
- [ ] **Step 2: Write the failing behaviour tests**, `// AC: CE-21.1` on the rename ones:
  - three commits, the last renaming the file with `id:` rewritten and a link in another file
    updated: newest first, `R` with `from` on the rename, `M`, `A`, all three paths right;
  - the same after two renames A → B → C;
  - `Page { skip: 1, max: 1 }` gives the middle commit; `skip` past the end gives `[]`;
  - a path never committed gives `[]`; a repository before its first commit gives `[]`;
  - `Approved-by: Jann` and `Co-Authored-By: …` come back as trailers in order;
  - a rename made on a branch and merged into `main` with `--no-ff`: the history on `main`
    still lists the commits before the rename (spec §7; if git loses them, record it as a
    finding for Jann, do not weaken the test);
  - a merge that resolves a conflict in the file is listed, with its change against the first
    parent.
- [ ] **Step 3: Run** `cargo nextest run -p theos-vcs history` — expect FAIL.
- [ ] **Step 4: Implement** with one `git log --follow --find-renames --diff-merges=first-parent
  --name-status -z --date=raw --format=<above> --skip=<n> --max-count=<n> -- <path>` through
  `read_args`. git's "does not have any commits yet" (exit 128 on an unborn branch) is `Ok(vec![])`:
  check `rev-parse --verify --quiet HEAD` first instead of matching stderr text.
- [ ] **Step 5: Run** the tests — expect PASS. **Commit** `feat(vcs): read a file's history across
  renames` (`Refs: CE-21.1`).

### Task 8: Reads that ignore the user's git settings

**Files:**
- Test: `crates/theos-vcs/tests/vcs/settings.rs`
- Modify: only `read_args.rs` if a setting gets through

- [ ] **Step 1: Write the failing test** `user_settings_do_not_change_the_reads`: one repository
  with a staged rename, an unstaged edit, an untracked file and two commits; read `status`,
  `history` and `read(Head)` once; then set in the repository's `.git/config`:
  `color.ui=always`, `color.status=always`, `log.showSignature=true`, `core.quotePath=true`,
  `status.renames=false`, `status.showUntrackedFiles=no`, `status.branch=false`,
  `diff.renames=false`, `diff.external=false-diff-tool`, `log.follow=false`,
  `log.decorate=full`, `format.pretty=oneline`, `i18n.logOutputEncoding=ISO-8859-1`,
  `core.precomposeUnicode=false`; read again; assert equal answers.
- [ ] **Step 2: Run** it; for each setting that changes an answer, add the counter-option to
  `read_args` (not to a single call site). Expect PASS.
- [ ] **Step 3: Commit** `test(vcs): reads ignore the user's git settings`.

### Task 9: `diff::words` in `theos-content`

**Files:**
- Create: `crates/theos-content/src/diff/mod.rs`, `crates/theos-content/src/diff/words.rs`
- Modify: `crates/theos-content/src/lib.rs` (`pub mod diff;`)

**Interfaces:**
- Produces: `pub struct WordDiff { pub ops: Vec<WordOp> }`,
  `pub enum WordOp { Equal { old: Span, new: Span }, Delete { old: Span }, Insert { new: Span } }`,
  `pub fn words(old: &str, new: &str) -> WordDiff` (spans are P02's `Span`, byte offsets).

- [ ] **Step 1: Write the failing tests**, `// AC: CE-21.2`:
  - `one_changed_word_is_one_delete_and_one_insert`: "under an open sky." → "under an opened
    sky." gives `Equal("under an")`, `Delete("open")`, `Insert("opened")`, `Equal("sky.")`
    (assert the texts behind the spans);
  - `a_rewrapped_paragraph_has_no_change`: the same words with `\n` instead of some spaces → a
    single `Equal`;
  - `a_no_break_space_joins_words`: "a\u{a0}b" → "a\u{a0}c" is one changed word;
  - `vertical_tab_and_form_feed_split_words` (git's `[:space:]`);
  - empty → text is all `Insert`; text → empty is all `Delete`; both empty → no ops;
  - `reconstruction` (`proptest`, texts of words from a small alphabet joined by random
    whitespace): the new text equals the `Equal`/`Insert` spans of the new text with the
    whitespace between them taken from the new text; likewise for the old text with
    `Equal`/`Delete`; no span starts or ends on whitespace; neighbouring ops are never the same
    kind.
- [ ] **Step 2: Run** `cargo nextest run -p theos-content diff` — expect FAIL.
- [ ] **Step 3: Implement**: tokens are maximal runs of chars not in `[' ', '\t', '\n', '\x0b',
  '\x0c', '\r']`, kept with their byte ranges; `similar::capture_diff_slices_deadline(
  Algorithm::Myers, &old_tokens, &new_tokens, Some(Instant::now() + Duration::from_secs(2)))`
  over the token texts; map each `DiffOp` to spans from its first token's start to its last
  token's end; `Replace` becomes `Delete` then `Insert`; merge neighbours of the same kind.
- [ ] **Step 4: Run** — expect PASS. **Commit** `feat(content): diff texts word by word`
  (`Refs: CE-21.2`).

### Task 10: `diff::fields` and `diff::record`

**Files:**
- Create: `crates/theos-content/src/diff/fields.rs`, `crates/theos-content/src/diff/record.rs`

**Interfaces:**
- Consumes: Task 9; P02's `Record::parse`, `Record::text`, `front_matter()`, `body_span()`,
  `Field`, `Value::to_raw()`, `RawValue`.
- Produces the types of spec §5.7 (as changed by the builder check): `FieldChange { key, delta }`,
  `FieldDelta::{Added { new: Field }, Removed { old: Field }, Changed { old: Field, new: Field,
  items: Option<ItemDelta> }}`, `ItemDelta { added: Vec<String>, removed: Vec<String> }`,
  `fn fields(old: &FrontMatter, new: &FrontMatter) -> Vec<FieldChange>`,
  `RecordDiff::{Record { old: String, new: String, fields: FieldsDiff, body: WordDiff,
  layout_only: bool }, Text { old: String, new: String, words: WordDiff }}`,
  `FieldsDiff::{Fields(Vec<FieldChange>), Text(WordDiff)}`,
  `fn record(old: Option<&str>, new: Option<&str>) -> RecordDiff`.

- [ ] **Step 1: Write the failing tests**, `// AC: CE-21.2`, on records copied from the data
  model's §14 examples (synthetic changes marked as such in the test):
  - `a_changed_scalar_shows_old_and_new`: `title` changed → one `Changed` whose old and new
    fields' source text (via the spans into `old`/`new`) are the two titles;
  - added and removed fields; order: new record's order, then removed fields in old order;
  - a list field with one item added and one removed → `items` with both;
  - `[a, b]` → block list `- a` / `- b`, `'x'` → `x`, a comment edited → no field change and
    `layout_only: true`; a re-wrapped body alone → `layout_only: true`;
  - CRLF on one side, LF on the other, same content → no change, `layout_only: true`, and the
    returned `old`/`new` are LF (Review Focus 4);
  - a YAML error on one side → `FieldsDiff::Text`;
  - a text without `---` lines → `RecordDiff::Text`;
  - `record(None, Some(t))` → every field `Added`, body all `Insert`; `record(Some(t), None)` the
    reverse;
  - a field whose value changes from scalar to list is `Changed`, `items: None`;
  - body spans are offsets into the whole LF record text (the first body word's span starts
    after the closing `---` line).
- [ ] **Step 2: Run** — expect FAIL.
- [ ] **Step 3: Implement**: compare fields by `to_raw()`; `items` only when both values are
  lists (multiset difference by item text); `layout_only` = texts differ and no field and no
  word changed; a side that fails `Record::parse` makes the whole diff `Text`.
- [ ] **Step 4: Run** — expect PASS. **Commit** `feat(content): diff records by field and by word`
  (`Refs: CE-21.2`).

### Task 11: `changes` in `theos-studio`

**Files:**
- Create: `crates/theos-studio/src/repo_path.rs` (P05's `project_path` moved from `staged.rs`,
  `pub(crate)`), `crates/theos-studio/src/changes.rs`
- Modify: `crates/theos-studio/src/staged.rs` (uses `repo_path`), `crates/theos-studio/src/lib.rs`
- Test: `crates/theos-studio/tests/studio/changes.rs`

**Interfaces:**
- Consumes: Task 6 (`status`, `Entry`, `Change`, `Conflict`, `Branch`), Task 5 (`read`), Task 4
  (`Cancel`); `staged::read`'s `top`, `place`, `folding` (or `repo::top`, `repo::place` and
  `Folding` directly).
- Produces the types of spec §5.8: `Changes { branch, records: Vec<RecordChange>, files:
  Vec<Entry> }`, `RecordChange { id, kind, title, path, from, repo_path, repo_from, staged,
  unstaged, untracked, conflict }`, and `pub fn changes(workspace: &Workspace, git: &Git,
  cancel: &Cancel) -> Result<Changes, StudioError>`.

- [ ] **Step 1: Write the failing tests** with `Repo::committed("")` and `Repo::committed("content")`
  (project in a subfolder) from `tests/studio/repo.rs`:
  - an edited truth `T-900` → one `RecordChange { id: "T-900", kind: Some("truth"), staged:
    Unchanged, unstaged: Modified, .. }` with its title;
  - an unstaged rename of `T-901` to `T-902` with `id:` rewritten → one entry, `id: "T-902"`,
    `from: Some(<old project path>)`, `repo_from` set;
  - a deleted record → its id, kind and title from HEAD's bytes;
  - a new untracked record → `untracked: true`;
  - `theos.toml` and `.theos/settings.toml` changed, and a file outside the project folder →
    `files`;
  - a `.md` file without front matter → `files`;
  - P03's cache files under `.theos/cache` unchanged (byte compare) after `changes` (ruling 9:
    no index sync, no cache write);
  - `Repo::committed("content")` with a change above the project folder → `files`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio changes` — expect FAIL.
- [ ] **Step 3: Implement** (spec §5.8 and ruling 9): classify by the newest version's bytes
  (`WorkTree`, else `Index`, else `Head`) with `Record::parse` and the registry's kind lookup;
  map paths with `repo_path::project_path`.
- [ ] **Step 4: Run** `cargo nextest run -p theos-studio` — expect PASS (P05's staged tests
  too, after the move). **Commit** `feat(studio): list changes by record`.

### Task 12: `diff` in `theos-studio`, and the word-diff oracle

**Files:**
- Create: `crates/theos-studio/src/diff.rs`
- Test: `crates/theos-studio/tests/studio/diff.rs`; fixture pairs in
  `crates/theos-studio/tests/fixtures/word-diff/` (`<name>.old.md`, `<name>.new.md`, synthetic,
  marked in a README in that folder)

**Interfaces:**
- Consumes: Tasks 5, 7 (`Commit`), 9, 10, 11 (`RecordChange::repo_path`).
- Produces: `pub enum Side<'a> { Staged, Unstaged, Commit(&'a Commit), Between(Version<'a>,
  Version<'a>) }`, `pub enum FileDiff { Record(RecordDiff), Text { old: String, new: String,
  words: WordDiff }, Binary }`, `pub fn diff(workspace: &Workspace, git: &Git, path: &[u8],
  from: Option<&[u8]>, side: Side<'_>) -> Result<FileDiff, StudioError>`.

- [ ] **Step 1: Write the failing tests**, `// AC: CE-21.2`:
  - `Staged`: a staged title change and one changed body word → `Record` with one field
    `Changed` and one word `Delete`/`Insert`;
  - `Unstaged`: an edit after staging shows only the edit since the index;
  - `Commit(c)` for a commit from `theos_vcs::history`, also for the rename commit (parent side
    read at `c.from`);
  - `Between(Rev("main"), Rev("agent/s-04"))` on a branch made in the test;
  - a non-UTF-8 file → `Binary`; `theos.toml` → `Text`;
  - **oracle**: for every fixture pair, the multisets of deleted and inserted words from
    `theos_content::diff::words` equal those of `git diff --no-index --word-diff=porcelain
    --word-diff-regex='[^[:space:]]+' old new` (lines starting `-` and `+`, split on
    whitespace). The pairs are written so the alignment is unique (each changed word differs
    from its neighbours); at least: one word changed, one sentence rewritten, a paragraph
    re-wrapped, a word inserted at the start and at the end, punctuation changed (`sky.` →
    `sky,`).
- [ ] **Step 2: Run** — expect FAIL. **Step 3: Implement** per spec §5.8. **Step 4: Run** —
  expect PASS.
- [ ] **Step 5: Commit** `feat(studio): diff a record between any two versions` (`Refs: CE-21.2`).

### Task 13: `history` in `theos-studio` (CE-21.1), docs

**Files:**
- Create: `crates/theos-studio/src/history.rs`
- Modify: `crates/theos-studio/src/lib.rs` (module docs: "P46 adds …"), `crates/theos-vcs/src/lib.rs`
  (module docs), `crates/theos-vcs/README.md` ("What P46 provides" table, "Who uses what" rows
  for P47, P07, P13 from spec §6.2)
- Test: `crates/theos-studio/tests/studio/history.rs`

**Interfaces:**
- Consumes: Task 7, Task 11 (`repo_path`), P03's `Index::record(id) -> Vec<RecordSummary>`,
  P06's `Rename { id, to }` and `Workspace::run`.
- Produces: `pub struct HistoryEntry { pub commit: Commit, pub path: String, pub id: String }`,
  `pub fn history(workspace: &Workspace, git: &Git, id: &str, page: Page, cancel: &Cancel)
  -> Result<Vec<HistoryEntry>, StudioError>`.

- [ ] **Step 1: Write the failing tests**, `// AC: CE-21.1`, on `Repo::committed("")`:
  - `a_record_renamed_with_the_rename_command_keeps_its_history`: commit an edit of `T-900`,
    run `Rename { id: "T-900", to: "T-903" }` through `Workspace::run`, commit; `history("T-903")`
    lists three entries, newest first, ids `T-903`, `T-900`, `T-900`, paths matching;
  - the same for the smallest record of the fixture (`C-900`, or the data model's smallest
    example if smaller);
  - a rename by `git mv` plus a hand edit of `id:`;
  - a record moved to another folder (`world/synthetic-low/…` to another floor folder);
  - two renames in a row;
  - a record that exists only in the working tree → `[]`;
  - an unknown id → `[]` (not an error).
- [ ] **Step 2: Run** — expect FAIL. **Step 3: Implement** per spec §5.8 (the id in a commit is
  the file name without `.md`; the path through the git index under `Folding`). **Step 4: Run**
  `cargo nextest run --workspace` — expect PASS.
- [ ] **Step 5:** update the READMEs and module docs; run `cargo xtask ac-trace --phase P46` —
  expect CE-21.1, CE-21.2 and CE-21.3 each with at least one passing test.
- [ ] **Step 6: Commit** `feat(studio): follow a record's history by id` (`Refs: CE-21.1`).

## After the tasks

- Final review loop (CLAUDE.md), then `cargo xtask gate` on Windows, then the PR
  `feat(vcs): git read (P46)` with the labels `windows` and `macos` (Tasks 3 and 4 hold per-OS
  code).
- In `docs/status.md` on this branch: P46 `in-progress` with the plan; the Spec PR column gets
  `#52, #58`.
