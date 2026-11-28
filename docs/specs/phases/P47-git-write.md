# P47 · Git write — phase spec

Status: draft · 28 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P47, umbrella version 8. Version 5 (#33) split the old
"Command layer and git" into P06, P46 Git read and P47 Git write. Scope and acceptance criteria
come from there; this spec decides how. It starts from P05's interfaces (P05 spec §5.2, §6), P06's
repo lock and command layer (P06 spec §5.7, §5.9, §6.2, §11) and P46's runner, reads and hand-offs
(P46 spec §6.2, §7; `crates/theos-vcs/README.md`, "Who uses what").

## 1. Goal

After P47, the editor can change git through `theos-vcs` and `theos-studio`, by record:

- **Stage and unstage** a record by all of its paths (both paths of a rename), or many at once.
- **Discard** a record's unstaged changes through P06's command layer, so Ctrl+Z brings them back.
- **Commit** the staged snapshot with a message and trailers, after the same staged check as P05's
  hook: errors block, warnings come back with the commit (CE-22.1).
- **Pull:** fetch, then merge the upstream. A conflicted pull is left for the merge tool with its
  conflicts listed by record; a merge that the hook refuses is left for the user to fix and commit.
- **Push** to the upstream. A failed push shows git's message whole; credentials go only through
  the user's credential helper, and git never prompts on a terminal (CE-22.3).

Every git write that changes the working tree or the index runs under P06's repo lock. The Changes
screen (P13, `docs/ui/screens/08-git.html`) is the first user.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-22 Git write and repo lock** (continued from P06)
- CE-22.1 Commit is blocked on errors in the staged snapshot and allowed with warnings.
- CE-22.3 Push errors show git's message; credentials only through the system helper.

## 3. Scope

**In:**
- in `theos-vcs`:
  - a run that returns git's exit code, stdout and stderr together (§5.2), and a finished run
    that keeps its result when a cancel meets its exit (§5.2);
  - one place for the options of every write (§5.2);
  - staging and unstaging by paths (§5.3), commit with trailers (§5.5), fetch, merge, the merge
    state and aborting a merge (§5.6), push (§5.7);
  - the bytes a checkout would write for a path (§5.4);
  - the remote environment: no terminal prompts, credentials through the helper (§5.8);
  - housekeeping, which runs git's automatic maintenance on request (§5.9);
- in `theos-studio`:
  - stage and unstage by record (§5.3), the `Discard` command (§5.4), the checked commit (§5.5),
    pull and push by record (§5.6, §5.7), each under the repo lock where §5.10 says so.

**Out:**
- a command line for the writes (Jann, §4.1);
- worktrees, agent branches and the approval merge (P07; `crates/theos-vcs/README.md` lists them
  under P47, the umbrella and architecture §8 put them in P07);
- creating, switching and deleting branches; amend, rebase, cherry-pick, revert, stash, tags;
- the merge tool and the Changes screen (P13, CE-36.3), "Commit on every save" (P13, P14);
- the AI-suggested commit message on the Changes screen (P17);
- git LFS beyond what git's own filters do (G10 keeps binaries out of git).

## 4. Decisions

### 4.1 Decided by Jann (28 Nov 2026)

- **Pull is fetch, then merge.** Fetch runs without the repo lock and can be cancelled; the merge
  of the upstream runs under the lock: a fast-forward when possible, a merge commit when the
  branches diverged. The merged result is judged with the commit's staged check before the merge
  commit is made (§5.6, ruling 12). Conflicts are left in
  the working tree, listed by record, for P13's merge tool. The user's `pull.rebase`, `pull.ff`,
  `merge.ff` and `merge.autoStash` settings are not honoured (§5.6).
- **Discard is undoable.** It runs through P06's command layer on the human route, so its journal
  entry restores the discarded bytes (CE-20.2). Its verdict never refuses it (§5.4).
- **Library only.** No `theos commit`, `pull` or `push`. Terminal users keep git, and P05's hook
  guards their commits. The ACs are proven by library tests through real git.

### 4.2 Probes (28 Nov 2026, Windows 11, git 2.56.1.windows.1)

- **Unstage before the first commit.** `git restore --staged -- f.md` fails with `fatal: could not
  resolve 'HEAD'`; `git rm --cached -- f.md` unstages it. With a commit, `git reset -q HEAD
  --pathspec-from-file=- --pathspec-file-nul` unstages.
- **Paths from stdin.** `git add`, `git reset` and `git rm --cached` take
  `--pathspec-from-file=- --pathspec-file-nul`, so any number of paths fits (Windows limits a
  command line to 32 767 characters).
- **Staging a rename.** A file renamed on disk shows as ` D i.md` + `?? i2.md`;
  `git --literal-pathspecs add -A -- i.md i2.md` stages it as `R  i.md -> i2.md`.
- **Local edits under a pull.** `git merge` of an upstream that changes a file with uncommitted
  edits refuses: "Please commit your changes or stash them before you merge. Aborting", and
  changes nothing. `git merge --autostash` merges, then leaves the file conflicted (`UU`) **and**
  a stash entry behind, with instructions to `git stash drop` or `git reset --hard`.
- **The hook refuses a merge.** With a `pre-merge-commit` hook that exits 1, a diverged
  `git merge --ff --no-edit` exits 1 with the hook's stderr and "Not committing merge; use 'git
  commit' to complete the merge."; `MERGE_HEAD` stays, the merged changes are staged. A later
  `git commit` records the merge with both parents.
- **Trailers and comment lines.** `git commit --cleanup=whitespace -F - --trailer "Approved-by:
  Jann"` with a message that has a line starting with `#` puts the trailer **above** that line:
  git takes `#` lines for comments when it looks for the trailer block. With
  `-c core.commentChar=auto` (deprecated, removed in git 3.0) the trailer goes last and the `#`
  line stays. A comment character that the message does not use does the same (§5.5).
- **The bytes of a checkout.** With `core.autocrlf=true`, `git cat-file --filters
  --path=e.md <blob>` gives `a\r\nb\r\n` for a blob holding `a\nb\n`: the bytes `git restore`
  writes. `cat-file --batch --filters` needs a path per object and fails on a bare object id.
- **No prompt.** `GIT_TERMINAL_PROMPT=0 git push https://example.invalid/x.git HEAD` ends at once
  with git's own message ("Could not resolve host").

### 4.3 Approaches considered

| Part | Chosen | Not chosen, and why |
|---|---|---|
| Git access | The git command line through P05's runner (G6) | `gix`, `git2` (G6) |
| Pull | `fetch`, then `merge --ff --no-edit --no-autostash` of the upstream (Jann) | `git pull` with the user's settings: a rebase stops per commit and runs no hook, so rebased commits skip the check; `--ff-only`: a teammate's push blocks pulling until it is solved outside the editor |
| Local edits that a pull would overwrite | Refuse before merging, naming the records (§5.6) | `--autostash`: leaves a conflict and a stash behind (probe); letting git refuse: its message names files, not records |
| Discard | A command on the human route, undoable (Jann) | `git restore` (irreversible) |
| The bytes discard writes | `cat-file --filters --path=<path>`, the bytes of a checkout | P46's `read(Index)`: the raw blob, which differs from what git writes under `core.autocrlf`, `.gitattributes` `eol` or a smudge filter |
| Commit checks | `check_staged` first, then `git commit` with the hooks running | `--no-verify` after our check: a user's other hooks (`commit-msg`, signing) would be skipped, and a change to the index between our check and the commit would go through unchecked |
| Trailers | `git commit --trailer`, with a comment character the message does not use | Appending trailer lines to the message ourselves (re-implements git's trailer block rules); `core.commentChar=auto` (deprecated) |
| No prompts | `GIT_TERMINAL_PROMPT=0`; ssh in batch mode when the user set no ssh command; stdin empty | A terminal prompt: the editor has no terminal, and on Unix git runs in its own process group, where reading the terminal stops it (SIGTTIN, P46 §7) |
| Credentials | The user's credential helper, as git configures it (G6) | Any credential handling in theos (G6, G14) |
| Automatic maintenance | Off for every write (`gc.auto=0`, `maintenance.auto=false`); `housekeeping` runs it in the foreground on request | git's detached `gc --auto`: on Windows the job of the run ends it half-way (P46 ruling R4-2); letting it outlive the job needs a breakaway flag on every run |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Crate | Tool or crate | Custom code, and why |
|---|---|---|---|
| Full-output run, write options | `theos-vcs` | P05/P46's runner | A second return shape: push and merge exit 1 with an answer on stdout |
| Stage, unstage | `theos-vcs` | `git add -A`, `git reset`, `git rm --cached`, `--pathspec-from-file` | — |
| Checkout bytes | `theos-vcs` | `git cat-file --filters --path=` | — |
| Commit | `theos-vcs` | `git commit -F - --trailer`, `git rev-parse` | The choice of a comment character |
| Fetch, merge, merge state, abort | `theos-vcs` | `git fetch`, `git merge`, `git merge --abort`, `git rev-parse -q --verify MERGE_HEAD`, `git diff --name-only -z` | — |
| Push | `theos-vcs` | `git push --porcelain` | A reader for `--porcelain`'s documented per-ref lines |
| Remote environment | `theos-vcs` | git's `GIT_TERMINAL_PROMPT`, `GIT_SSH_COMMAND`, ssh's `BatchMode` | — |
| Housekeeping | `theos-vcs` | `git maintenance run --auto` | — |
| By record | `theos-studio` | P46's `changes`, P06's command layer and repo lock, P05's `check_staged` | — |
| Credential test | tests | `std::net::TcpListener` as a tiny HTTP server, a credential helper script | No crate needed for one 401 and one header check |

No new dependency.

### 5.2 Running a write

```rust
/// What a git command answered, success or not.
pub struct Output { pub code: Option<i32>, pub stdout: Vec<u8>, pub stderr: String }

impl Git {
    /// As `run_cancellable`, but a failing exit is an answer, not an error. `Err` only for
    /// `Cancelled`, `Io`, and a git that could not start.
    pub fn output(&self, args: &[&OsStr], cwd: &Path, stdin: Option<&[u8]>,
                  env: &[(&str, &str)], cancel: &Cancel) -> Result<Output, GitError>;
}
```

- **One spawn path** (P46 ruling 3): `run_cancellable` is `output` plus the mapping of a failing
  exit to `GitError::Failed`.
- **A finished run keeps its result.** When a cancel meets git's exit, the exit wins: `wait`
  returns `Exited` whenever git's status has arrived, before or after the cancel is seen. Only a
  git still running when the cancel is seen is stopped and gives `Cancelled`. This holds for every
  run; a read that finished anyway gives its answer, which is never wrong. It answers P46's open
  point: a push or commit that git finished reports its real result.
- **Write options**, one list like P46's read options, for every write:
  - `--literal-pathspecs`;
  - `-c color.ui=never`, `-c core.quotePath=false`, `-c advice.*` left as the user has it (git's
    messages are shown, not parsed);
  - `-c gc.auto=0`, `-c maintenance.auto=false` (§5.9);
  - no `--no-optional-locks`: writes take git's locks.
- **What is parsed, and what is shown.** theos parses only documented machine formats
  (`--porcelain`, `-z`, `rev-parse`), never git's messages. Messages are shown as git wrote them,
  in the user's language.
- **`index.lock`.** When another git holds the index (a terminal, an IDE), git fails with "Unable
  to create '…/index.lock': File exists". theos maps a failing write whose stderr names
  `index.lock` to `GitError::IndexLocked { stderr }` and never deletes the file: only the user
  knows whether that other git is still running. This is the one message theos looks into; the
  error carries git's text whole.
- **Not cancellable:** stage, unstage, commit, merge and abort. They are local and short, and
  stopping git half-way through them can leave `index.lock` behind. They pass a flag nobody sets.
  Fetch and push take the caller's `Cancel`.

### 5.3 Stage and unstage

```rust
// theos-vcs
pub fn stage(git: &Git, top: &Path, paths: &[&[u8]]) -> Result<(), GitError>;
pub fn unstage(git: &Git, top: &Path, paths: &[&[u8]]) -> Result<(), GitError>;

// theos-studio
pub fn stage_records(workspace: &mut Workspace, git: &Git, records: &[&RecordChange],
                     files: &[&Entry]) -> Result<(), StudioError>;
pub fn unstage_records(workspace: &mut Workspace, git: &Git, records: &[&RecordChange],
                       files: &[&Entry]) -> Result<(), StudioError>;
```

- Paths are git's bytes relative to the top, as `status` and `RecordChange` give them
  (`repo_path`, `repo_from`). The functions take no other paths, so `..` and absolute paths
  cannot reach git (P46 hand-off).
- **Stage:** `git add -A --pathspec-from-file=- --pathspec-file-nul` with the paths on stdin.
  `-A` stages a deletion too, so both paths of a rename make one staged rename (probe).
- **Unstage:** with a `HEAD`, `git reset -q HEAD --pathspec-from-file=- --pathspec-file-nul`;
  before the first commit, `git rm -q --cached --ignore-unmatch` with the same paths (probe).
- **By record:** each `RecordChange` adds its `repo_path` and, for a rename, its `repo_from`. A
  conflicted record is refused with `StudioError::Conflicted { id }`: staging a conflicted path
  marks it resolved, which is the merge tool's step (P13). Untracked records stage like any other.
- **Stage all / unstage all** are the same calls with every entry of `Changes`.
- Each call takes the repo lock (§5.10) through the workspace, as P06's pipeline does, so a
  workspace that already holds it gets `Reentrant` at once instead of waiting for itself. An empty
  list runs no git.

### 5.4 Discard (undoable)

```rust
// theos-vcs
/// The bytes `git checkout` would write for `path` from the index, after git's filters.
pub fn checkout_bytes(git: &Git, top: &Path, path: &[u8]) -> Result<Content, GitError>;

// theos-studio, a command of the command layer
pub struct Discard { /* built from a RecordChange or an Entry */ }
impl Discard {
    pub fn record(workspace: &Workspace, git: &Git, record: &RecordChange) -> Result<Discard, StudioError>;
    pub fn file(workspace: &Workspace, git: &Git, entry: &Entry) -> Result<Discard, StudioError>;
}
```

- **What it discards:** the unstaged change only, the working tree back to the index; what is
  staged stays staged. This is git's "discard" in every client and what the Changes screen shows
  next to "Unstage".
  - A modified record: its file gets the index's checkout bytes.
  - An unstaged deletion: the file is created again from the index.
  - An unstaged rename (paired by P46's status): the old path is created again from the index,
    the new path is removed.
  - An untracked record: its file is removed. The journal keeps its bytes, so undo creates it
    again.
  - A record with only staged changes: nothing to discard; the plan is empty.
  - A conflicted record: refused (`StudioError::Conflicted`); `abort_merge` (§5.6) or the merge
    tool deal with it.
  - A path the command layer may not write is refused with `StudioError::NotDiscardable { path,
    why }`, and nothing is written: a path outside the project folder (also the old path of a
    rename from outside it), and every path P06's boundary rules refuse (a `.gitignore`, a hidden
    file on Windows). The message says that `git restore` in a terminal discards it. There is no
    second, irreversible way to discard in the editor.
- **The bytes:** `checkout_bytes` resolves the index object with P46's rules (`:0:<path>`, mode
  from `ls-files -s`) and reads it with `git cat-file --filters --path=<path> <object>`, so
  `core.autocrlf`, `.gitattributes` `eol` and smudge filters give the bytes `git restore` would
  write (probe). A link or a submodule is `NotAFile` and is refused.
- **Git is read when the command is built,** before the pipeline: `Discard::record` reads the
  bytes and keeps them. The plan writes exactly those bytes, never normalised text, as undo does
  (P06 §5.7). The base check of P06 §5.3 compares the working file with what was read when the
  command was built, so an edit in between makes the command fail with `Stale`, as for any command.
- **The verdict never refuses a discard.** The command declares that it restores a state, and the
  pipeline then allows every count (P06 §5.5 counts the findings; a discard's are reported in
  `Outcome.findings` and `introduced`, but none blocks). Why: the index version may hold an error
  that the working copy fixed, and the user asked for exactly that version back; refusing would
  leave no way to discard but a terminal. The builder adds this as a property of the command
  (a `Command` method with a default that judges), not as a new route. Only the verdict is
  skipped: the path and boundary rules of P06 stay.
- **Undo and redo** are P06's: undo writes the discarded working bytes back, with P06's undo
  verdict; redo discards again.
- It runs on the human route, so it holds the repo lock like every command.

### 5.5 Commit (CE-22.1)

```rust
// theos-vcs
pub struct Trailer { pub key: String, pub value: String }
pub fn commit(git: &Git, top: &Path, message: &str, trailers: &[Trailer]) -> Result<CommitId, GitError>;

// theos-studio
pub struct Committed { pub id: CommitId, pub check: StagedCheck }   // check: warnings only
pub fn commit(workspace: &mut Workspace, git: &Git, message: &str, trailers: &[Trailer])
    -> Result<Committed, CommitError>;

#[non_exhaustive]
pub enum CommitError {
    Blocked(StagedCheck),      // errors in the staged snapshot; nothing was committed
    NothingStaged,
    Message(String),           // empty after trimming, or no free comment character
    Conflicted(Vec<String>),   // record ids or paths with unresolved conflicts
    Lock(LockError),
    Git(GitError),             // git's message whole: a hook, signing, `index.lock`
    Studio(StudioError),
}
```

The studio's commit, in this order:

1. Take the repo lock (purpose `commit`).
2. Refuse an empty message (after trimming) with `Message`.
3. Read the staged snapshot. No staged change and no merge in progress gives `NothingStaged`.
   An unresolved conflict gives `Conflicted`.
4. Run P05's `check_staged` (§6.1). Any error gives `Blocked` with the whole check; nothing is
   committed. Warnings do not block.
5. The check has ended its index transactions (P05 §6, P06 §11); git starts only now, because the
   hook syncs the same cache.
6. `theos_vcs::commit`: `git commit -F - --cleanup=whitespace --trailer <key>: <value>…` with the
   message on stdin and the write options. The hooks run: P05's `pre-commit` judges the snapshot
   again (it catches a change to the index made by another program after step 4) and the user's
   own hooks run as they always do. Signing follows the user's settings. A refused hook or a
   failed signature is `CommitError::Git` with git's message.
7. Read the new commit with `git rev-parse HEAD` and return `Committed` with the check's
   warnings.
8. Release the lock. The index is not synced here: the hook synced it, and P11's watcher sees the
   changed `HEAD`.

- **Message:** `--cleanup=whitespace` keeps lines that start with `#` (a record id, a heading).
  theos passes `-c core.commentChar=<c>` with the first of `# ; @ ! $ % ^ & | :` that starts no
  line of the message, so git puts the trailers last (probe). A message that starts lines with
  all ten is refused with `CommitError::Message` naming the reason, as is an empty one. `theos_vcs::commit` refuses a trailer key that is not `^[A-Za-z0-9-]+$` or a value
  with a line break.
- **A merge in progress** (after a conflicted or refused pull): the commit concludes it, as
  `git commit` does; git records both parents. `merge_state` (§5.6) gives git's prepared message
  for the caller to offer.
- **Who adds trailers:** callers. P07 adds `Approved-by:` to its approval merges, and the
  generator phases add generation details to dialogue commits (data model §7, dialogue).
- **Identity:** the user's `user.name` and `user.email`. When git refuses for a missing identity,
  its message ("Please tell me who you are") is shown; theos never sets an identity.

### 5.6 Pull

```rust
// theos-vcs
pub fn fetch(git: &Git, top: &Path, remote: &str, cancel: &Cancel) -> Result<(), GitError>;
pub enum Merged { UpToDate, FastForward { to: CommitId }, Staged, Conflicts }
// Staged: MERGE_HEAD set, the merged result staged, not committed
pub fn merge(git: &Git, top: &Path, upstream: &str) -> Result<Merged, GitError>;
pub struct MergeState { pub head: CommitId, pub message: String }
pub fn merge_state(git: &Git, top: &Path) -> Result<Option<MergeState>, GitError>;
pub fn abort_merge(git: &Git, top: &Path) -> Result<(), GitError>;

// theos-studio
pub enum Pulled {
    UpToDate,
    FastForward { to: CommitId },
    Merged { id: CommitId, check: StagedCheck },   // check: warnings only
    Conflicts(Changes),                       // the conflicted records, for the merge tool
    Refused { check: StagedCheck, changes: Changes }, // errors in the merged result; the merge waits
}
pub fn pull(workspace: &mut Workspace, git: &Git, cancel: &Cancel) -> Result<Pulled, PullError>;
pub fn abort_merge(workspace: &mut Workspace, git: &Git) -> Result<(), StudioError>;

#[non_exhaustive]
pub enum PullError {
    Detached, NoUpstream { branch: String },
    LocalChanges(Vec<RecordChange>, Vec<Entry>),   // uncommitted changes the merge would overwrite
    MergeInProgress,
    Lock(LockError), Git(GitError), Studio(StudioError),
}
```

The studio's pull:

1. Read `status`. A detached `HEAD` is `Detached`; a branch without an upstream is `NoUpstream`;
   a merge in progress is `MergeInProgress`.
2. **Fetch** the upstream's remote, without the repo lock: fetch writes objects and remote refs
   only, never the working tree or the index. It takes the caller's `Cancel` and the remote
   environment (§5.8). A cancelled fetch changes nothing the user sees.
3. Take the repo lock (purpose `pull`) and read `status` again: ahead and behind against the
   fetched upstream. Behind 0 is `UpToDate`.
4. **Local changes in the way:** the paths the upstream changed since the merge base
   (`git diff --name-only -z <base> <upstream>`, the merge base from `git merge-base`) are compared
   under the repository's `Folding` with every path that has a staged, unstaged or untracked
   change. Any overlap is `LocalChanges` with those records and files, and nothing is merged; the
   user commits or discards them first. When the pull is not a fast-forward (ahead and behind both
   above 0), every staged path also counts: git refuses a merge commit while anything is staged,
   even a file the upstream did not touch (builder probe, 28 Nov 2026). git's own refusal stays the
   backstop: its message is shown as `Git`.
5. **Merge:** `git merge --ff --no-commit --no-edit --no-autostash --no-squash <upstream>` with
   the write options. `--ff` and `--no-autostash` override the user's `merge.ff` and
   `merge.autoStash`; `--no-commit` stops before a merge commit is made (a fast-forward still
   moves the branch: `--no-commit` does not stop one).
   - a fast-forward: `FastForward` (no check runs; P05 §7 lists this, the upstream's commits were
     checked when they were made, and P49's check action guards the remote);
   - a clean merge: `Staged`. The studio judges the merged result itself with `check_staged`, as
     the commit does, so it is judged whether or not the user installed P05's hook. With errors:
     `Refused` with the check and the `Changes`; `MERGE_HEAD` stays and the merged result is
     staged, so the user fixes the records and commits (§5.5), or aborts. Without errors: the
     studio commits the merge with git's prepared message (`git commit --no-edit` with the write
     options; `pre-commit` runs, not `pre-merge-commit`) and returns `Merged` with the check's
     warnings;
   - conflicts (exit 1 with `MERGE_HEAD` and unmerged entries): `Conflicts` with P46's `changes`,
     whose `RecordChange.conflict` names each one (CE-36.3 is P13's).

   `theos_vcs::merge` tells these apart from the exit code, `MERGE_HEAD` and the unmerged entries
   (`ls-files -u -z`), never from git's message.
6. Release the lock. After a merge, the index syncs the changed paths (P03's `sync`), outside the
   lock, as after any outside change. P06's journal precondition keeps undo from overwriting what
   the pull wrote (P06 §6.2).

- **`merge_state`** reads `MERGE_HEAD` (`rev-parse -q --verify`) and git's `MERGE_MSG` (through
  `rev-parse --git-path MERGE_MSG`, so linked worktrees work).
- **`abort_merge`** runs `git merge --abort` under the repo lock: git puts back the state before
  the merge and keeps uncommitted changes that the merge did not touch. It is refused when no
  merge is in progress.
- **Remote and upstream:** both come from the branch's configuration, read with
  `git config --get` (`branch.<name>.remote`, `branch.<name>.merge`). P46's `Upstream.name`
  (`origin/main`) is not split, since a remote or a branch may contain `/`. theos never guesses
  another remote.

### 5.7 Push (CE-22.3)

```rust
// theos-vcs
pub enum Pushed { UpToDate, Updated { from: Option<CommitId>, to: CommitId } }
pub fn push(git: &Git, top: &Path, target: &PushTarget, cancel: &Cancel) -> Result<Pushed, PushError>;
pub struct PushTarget { pub remote: String, pub branch: String, pub set_upstream: bool }

#[non_exhaustive]
pub enum PushError {
    Rejected { reason: Rejection, message: String },  // message: git's stderr, whole
    Failed { message: String },                       // auth, network, a remote hook: git's stderr, whole
    Git(GitError),                                    // cancelled, could not start
}
pub enum Rejection { NotFastForward, RemoteRefused, Other }

// theos-studio
pub fn push(workspace: &mut Workspace, git: &Git, cancel: &Cancel) -> Result<Pushed, StudioPushError>;
```

- **Target,** in git's own order for the push remote (`git help config`,
  `branch.<name>.pushRemote`):
  1. the remote: `branch.<name>.pushRemote`, else `remote.pushDefault`, else
     `branch.<name>.remote`, each read with `git config --get`;
  2. the branch on it: the upstream branch (`branch.<name>.merge`) when the push remote is the
     upstream's remote; the local branch's own name when it is another remote (a fork), as
     git's `push.default=simple` does in a triangular workflow;
  3. pushed as `git push --porcelain <remote> HEAD:refs/heads/<branch>`, so a user's
     `push.default` cannot send other branches.

  A branch with none of the three settings is pushed to the repository's only remote under its
  own name with `--set-upstream`; with no remote or several, the studio refuses with
  `NoUpstream { remotes }`, naming them. A detached `HEAD` is refused.
- **No repo lock.** A push changes neither the working tree nor the index; holding the lock
  across the network would block every command for as long as the remote takes. It takes the
  caller's `Cancel`.
- **The message (CE-22.3).** A failed push returns git's stderr whole, the remote's own lines
  (`remote: …`, a pre-receive hook's text) included, in `message`. theos adds nothing to it
  except the kind of rejection, read from `--porcelain`'s per-ref line (`!` with
  `[rejected] (non-fast-forward)` or `(fetch first)` is `NotFastForward`; `[remote rejected]` is
  `RemoteRefused`), so P13 can offer "Pull first". The `Display` of every variant ends with git's
  message.
- **Cancel:** a push cancelled while git runs may have reached the remote. The result is
  `Cancelled`, and the caller reads `status` again; P46's ahead and behind then come from the
  remote ref that git updated, or not.

### 5.8 Credentials and prompts (CE-22.3)

- **Through the helper only (G6).** theos never reads, stores, passes or logs a credential: no
  user name or token in a URL, an argument or the environment. git asks the user's configured
  `credential.helper` (Git Credential Manager on Windows, `osxkeychain` on macOS, `libsecret` or
  `store` on Linux), which may show its own window.
- **No terminal prompt.** Fetch and push run with:
  - `GIT_TERMINAL_PROMPT=0`: when no helper answers, git fails at once with its own message
    instead of asking on a terminal;
  - `GIT_SSH_COMMAND="ssh -o BatchMode=yes"` only when the user has set none of
    `GIT_SSH_COMMAND`, `GIT_SSH` and `core.sshCommand`: ssh uses the agent and the key files, and
    fails with its own message where it would ask for a passphrase or accept an unknown host key;
  - no askpass program: `GIT_ASKPASS` and `SSH_ASKPASS` are removed from git's environment and
    `-c core.askPass=` clears the setting, so an askpass inherited from the editor's environment
    (an IDE's terminal) cannot ask for a credential; `SSH_ASKPASS_REQUIRE=never`;
  - stdin empty.
- This settles P46's open point: on Unix git runs in its own process group (P46 §5.3), where a
  read from the terminal would stop it (SIGTTIN). With no prompt it never reads the terminal.
- The variables are set per call, never with `Git::with_env` on the shared `Git`.

### 5.9 Housekeeping

- Every write runs with `gc.auto=0` and `maintenance.auto=false`, so git never starts a detached
  `gc --auto` that the Windows job of the run would end half-way (P46 ruling R4-2).
- `housekeeping(git, top, cancel)` runs `git -c gc.autoDetach=false -c maintenance.autoDetach=false
  maintenance run --auto` in the foreground, in the run's job: git decides whether anything is due,
  as it would after its own commands. It takes no repo lock (git's maintenance has its own lock and
  never touches the working tree or the index).
- P13 calls it in the background after a commit or a pull; a failure is logged at `warn` and never
  shown as an error.

### 5.10 The repo lock

| Operation | Lock | Why |
|---|---|---|
| stage, unstage | yes | writes the index |
| discard | yes (human route) | writes the working tree |
| commit | yes, for the whole check and commit | writes the index and `HEAD`; P06 §6.2 |
| pull: fetch | no | objects and remote refs only; may wait on the network |
| pull: merge, abort | yes | writes the working tree, the index and `HEAD` |
| push | no | neither working tree nor index; may wait on the network |
| housekeeping | no | git's own lock |

- Purposes in the owner file: `stage`, `unstage`, `commit`, `pull`, `abort merge`, and the
  discard command's label. A waiter sees them in P06's `Busy` message.
- The lock is taken with P06's 10 s wait. A held lock is `Lock(Busy)`; nothing ran.
- A git in a terminal does not know the lock; git's `index.lock` keeps it and theos apart (§5.2).

### 5.11 Errors

- `GitError` (non-exhaustive) gets `IndexLocked { stderr }` (§5.2).
- `theos-vcs` adds `PushError` (§5.7). `theos-studio` adds `CommitError`, `PullError` and a push
  error wrapping `PushError` and `NoUpstream`, and `StudioError::Conflicted { id }`. Each has the
  `GitError` or `LockError` as its source, so P05's `report` prints each cause once.
- Messages from git are kept whole and never translated or shortened.
- No `unwrap` or `expect` outside tests.

### 5.12 Tests and fixtures

- **Git in tests** as in P05 §5.13 and P46 §5.10: the real `git`, `GIT_CONFIG_GLOBAL` on an empty
  file, `GIT_CONFIG_NOSYSTEM=1`, identities by `-c`. Remotes are local bare repositories
  (`git init --bare`); no test uses the network.
- **Records** are copies of the data model's examples and P02's fixtures. The error of CE-22.1 is
  a dangling link or a `never`-confirm violation as P05's hook tests build them; the warning is
  one of P04's warning rules. Synthetic text says so in the fixture.
- **Hostile user settings:** one test sets `pull.rebase=true`, `pull.ff=only`, `merge.ff=false`,
  `merge.autoStash=true`, `push.default=matching`, `commit.verbose=true`, `core.commentChar=;`,
  `color.ui=always`, `core.quotePath=true` and `gc.auto=1`, then stages, commits, pulls (fast
  forward and diverged) and pushes, and expects the same results as without them.
- **The credential test** (CE-22.3) needs no network and no new crate: a test thread serves HTTP on
  `127.0.0.1:0` with `std::net::TcpListener`, answers the first request with `401` and
  `WWW-Authenticate: Basic`, and records the `Authorization` header of the retry. The repository's
  `credential.helper` is a test script that writes a marker and answers a user name and token.
  The test proves the helper was asked and the header carries its answer. A second test without a
  helper proves the push fails within a few seconds with git's message (no prompt, no hang),
  also with `GIT_ASKPASS` set to a script that would answer (it is never run).
- **Per OS:** no new OS-specific code. The SSH batch mode and the prompt variables are the same
  on every OS. The tests run on all three in the weekly `platforms` run; the `wait` change
  (§5.2) touches code shared by both process-tree paths, so the PR gets the labels `windows` and
  `macos`.

### 5.13 How each AC is proven

| AC | Tests |
|---|---|
| CE-22.1 | **Through real git, in `theos-studio`:**<br>- a staged record with an error: `commit` returns `Blocked` with the finding, `HEAD` is unchanged and the index still holds the staged change;<br>- a staged record with only a warning: `commit` returns `Committed`, `HEAD` is the new commit with the message and trailers, and `check` holds the warning;<br>- the error only in the working tree, the staged version clean: allowed;<br>- the error staged, the working tree fixed but not staged: blocked;<br>- a staged deletion that leaves a dangling link: blocked;<br>- a merge left by a refused pull, its records fixed and staged: the commit concludes the merge with two parents.<br>**In `theos-vcs`:** `commit` with trailers and a message with `#` lines (trailers last, `#` lines kept); a refusing `pre-commit` script gives `GitError::Failed` with its stderr whole and no commit. |
| CE-22.3 | **Through real git, in `theos-vcs`:**<br>- a remote whose `pre-receive` hook prints a reason and exits 1: `Rejected { RemoteRefused }`, `message` contains the hook's lines;<br>- a remote ahead of the branch: `Rejected { NotFastForward }` with git's message;<br>- a remote path that does not exist: `Failed` with git's message;<br>- the credential helper test and the no-helper test of §5.12;<br>- a branch without an upstream and one remote: pushed with `--set-upstream`. |

Behaviour tests beyond the ACs: staging a paired rename gives one staged rename; unstaging before
the first commit; paths with spaces, `*`, `:`, a leading `-` and non-ASCII; more paths than fit on
one Windows command line; a conflicted record refused by stage and discard; discard of each kind of
§5.4 and its undo, with `core.autocrlf=true`; pull up to date, fast-forward, merged, conflicted,
refused by the check of the merged result, and refused for local changes, naming the record; abort; a
held `index.lock` gives `IndexLocked` and the file stays; a cancelled fetch and push; the lock held
by another process makes stage, commit and pull wait and then fail with `Busy`.

## 6. Interfaces

### 6.1 From P05, P06 and P46

- P05: `check_staged(&mut Workspace, &Git) -> Result<StagedCheck, StudioError>`, `blocks()`,
  `changed`; `staged::read` and `Staged`. The commit holds the repo lock while git runs the hook;
  the check never takes it and holds no index transaction when it returns.
- P06: `lock::acquire` and `LockError`; the command layer, the human route, the journal and its
  precondition; `Outcome`.
- P46: `Git::output` joins `run` and `run_cancellable` on one spawn path; `Cancel`; `status`,
  `Branch`, `Upstream`, `Entry`; `changes` and `RecordChange` with `repo_path` and `repo_from`;
  `read`'s index resolution, which `checkout_bytes` shares; the read options.

### 6.2 For other phases

| Phase | Uses |
|---|---|
| P13 | `stage_records`, `unstage_records`, `Discard`, `commit` with `Blocked` and the warnings (CE-36.1), `pull` with `Conflicts` for the merge tool (CE-36.3), `merge_state`, `abort_merge`, `push` with `Rejection::NotFastForward` for "Pull first", `housekeeping` after a commit or pull |
| P07 | `theos_vcs::commit` with `Trailer` for agent commits in a session worktree and `Approved-by:` on approvals; `Git::output`; `IndexLocked`. P07 adds worktrees and the approval merge, and decides when the agent route holds the repo lock (P06 §5.10) |
| P14 | A setting for "Commit on every save" calls `commit` (P13 decides the message) |
| P16, P17 | `Trailer` for generation details on dialogue commits (data model §7, dialogue) |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| A credential helper that opens a window (Git Credential Manager) while the editor waits | A push or fetch that seems to hang | It takes the caller's `Cancel`; P13 shows it running and offers Cancel |
| An ssh key with a passphrase and no agent | Push and fetch fail where a terminal would have asked | ssh's message is shown whole; the user adds the key to the agent. Users who set their own ssh command keep it |
| A user's `pre-commit` or signing that needs a terminal (pinentry-tty) | The commit fails | git's message is shown; GUI pinentry works. Not cancellable, so a hook that never ends blocks the commit; P13 shows it running |
| A pull with local changes in other files | Unstaged and untracked ones stay untouched and the merge goes ahead; staged ones refuse a pull that is not a fast-forward | git's behaviour (builder probe); `LocalChanges` names them before git runs (§5.6) |
| A fast-forward pull brings in a commit with an error | No hook runs on a fast-forward | P05 §7 accepted this; the next commit's check blocks on the whole snapshot; P49's check action guards the remote |
| Automatic maintenance turned off for theos's writes | A repository used only through the editor grows | `housekeeping` runs git's maintenance in the foreground (§5.9) |
| `cat-file --filters` and a smudge filter that needs the network (git LFS) | Discard is slow or fails | G10 keeps binaries out of git; a failure is git's message and nothing is written |
| The `wait` change for every run | A read that finished as it was cancelled returns its answer | The answer is complete and correct; callers that cancelled drop it |

## 8. Review Focus — inputs most likely to break it

- **Paths:** spaces, `*`, `?`, `[`, `:`, a leading `-`, non-ASCII (NFC and NFD), a case-only
  rename on a case-insensitive disk, a project in a subfolder with changes outside it, a path list
  longer than a Windows command line.
- **Stage and unstage:** a paired unstaged rename; a staged rename with an unstaged edit; a file
  staged then deleted on disk; an intent-to-add entry; no commit yet; a conflicted path.
- **Discard:** each case of §5.4; `core.autocrlf=true` and an `eol=crlf` attribute; a file edited
  between building the command and running it; undo after the file changed again; a record whose
  index version has an error.
- **Commit:** an empty or whitespace-only message; `#` and `;` lines; a trailer key with a space or
  a value with a line break; a merge in progress; no identity; a hook that prints a lot; a change
  staged by another program between the check and the commit.
- **Pull:** detached `HEAD`; no upstream; an upstream that is gone; local changes overlapping and
  not overlapping, staged and untracked; a hook refusing the merge; conflicts of each kind; a
  cancelled fetch; the user's `pull.*`, `merge.*` settings.
- **Push:** no upstream with zero, one, two remotes; an upstream under another name; rejected
  non-fast-forward and by a remote hook; an unreachable remote; no credential helper; a cancel
  during the push.
- **Lock:** each operation of §5.10 with the lock held by another process; the lock never held
  during fetch or push.

## 9. Rulings (Claude's calls)

1. Ruling: `check_staged` runs before `git commit`, and the hooks still run — our check gives the
   editor structured findings, the hook catches index changes made between the check and the
   commit, and the user's other hooks keep working — cost if wrong: the staged snapshot is judged
   twice per commit.
2. Ruling: a finished run keeps its result when a cancel meets its exit, for every run — a commit
   or push that git completed must not be reported as cancelled, and one rule is simpler than two
   — cost if wrong: a cancelled read sometimes returns an answer, which callers drop.
3. Ruling: stage, unstage, commit, merge and abort are not cancellable — they are local and short,
   and stopping git inside them can leave `index.lock` behind — cost if wrong: a slow hook cannot
   be stopped from the editor.
4. Ruling: fetch and push run without the repo lock — neither touches the working tree or the
   index, and holding the lock across the network would block every command — cost if wrong: a
   command and a fetch at the same time see remote refs change under them, which nothing reads
   for a decision.
5. Ruling: a pull refuses local changes that the upstream would overwrite, before merging, and
   names their records — the probe shows `--autostash` leaves a conflict and a stash behind, and
   git's own refusal names files — cost if wrong: the user commits or discards before pulling,
   which git requires anyway.
6. Ruling: discard writes the bytes of a checkout (`cat-file --filters`), not the raw blob — under
   `core.autocrlf` the raw blob differs from what git writes and would show as a new change — cost
   if wrong: none known; a filter that fails makes discard fail with git's message.
7. Ruling: the comment character of a commit is the first of a fixed list that starts no line of
   the message — git otherwise places trailers above a `#` line (probe), and `core.commentChar=auto`
   is deprecated — cost if wrong: a message using all ten characters as line starts is refused.
8. Ruling: push targets the upstream with an explicit refspec, and without one the only remote
   with `--set-upstream`; several remotes are refused — the user's `push.default` must not send
   other branches, and guessing among remotes could publish to the wrong one — cost if wrong: a
   user with several remotes sets the upstream once in a terminal.
9. Ruling: automatic maintenance is off for theos's writes and runs on request — a detached
   `gc --auto` would be ended half-way by the Windows job (P46 ruling R4-2) — cost if wrong: the
   repository grows until `housekeeping` or the user's own git runs maintenance.
10. Ruling: only `index.lock` is recognised in git's messages — it is the one failure the editor
    must treat differently (another git is running), and all other messages are shown as they
    are — cost if wrong: a git that words it differently gives `Failed` with the same text.
11. Ruling: worktrees and the approval merge stay in P07, although `crates/theos-vcs/README.md`
    lists them under P47 — the umbrella gives P47 "staging, committing and syncing", and
    architecture §8 ties worktrees and approval to agent sessions — cost if wrong: P07 builds
    them, as it would have used them first anyway. The build updates the README.
12. Ruling: the pull judges a clean merge itself (`merge --no-commit`, `check_staged`, then
    `git commit --no-edit`) instead of relying on P05's `pre-merge-commit` hook — the hook exists
    only where the user ran `theos hook install`, and the commit already judges for itself — cost
    if wrong: a user's own `pre-merge-commit` hook does not run on the editor's pulls
    (`pre-commit` runs instead).
13. Ruling: push follows git's push-remote order (`pushRemote`, `pushDefault`, the upstream's
    remote) — in a fork workflow the upstream's remote is the wrong place to publish — cost if
    wrong: none known; it is git's own rule.
14. Ruling: askpass programs are switched off for fetch and push — CE-22.3 allows credentials only
    through the system helper, and an IDE's askpass would be another channel — cost if wrong: a
    user who relies on an askpass instead of a helper must configure a helper.
15. Ruling: a discard the command layer cannot write (outside the project, a boundary path) is
    refused with a message pointing to `git restore` — one undoable way to discard, and P06's
    boundary rules stay — cost if wrong: such files are discarded in a terminal.

## 10. Left to the builder

- Module names inside the crates.
- How the `Command` trait marks a command whose verdict never refuses (§5.4), within P06's
  design: one method with a default.
- The reader of `push --porcelain`'s lines and of `ls-files -u -z`.
- The HTTP server of the credential test, within §5.12.
- Whether `stage_records` and `unstage_records` take `RecordChange`s or ids.
