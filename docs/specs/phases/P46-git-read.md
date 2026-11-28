# P46 · Git read — phase spec

Status: approved (#52) · 27 Nov 2026 · repo of the build: `theos-editor`
Changes: 27 Nov 2026 — process trees through a `win32job` job on Windows and a process group with
`nix`'s `killpg` on Unix, not `process-wrap` (§4.2, §4.3, §5.1, §5.3, §7, §9 R2, R3; Jann). Why:
the plan's re-probe measured `process-wrap`'s job object at about 70 ms more per git start on
Windows, which breaks §5.3's limit.
Changes: 28 Nov 2026 — the spec as built (build PR #62): the job assignment of a git that has
already exited (§5.3, §7), the read options' root-commit setting (§5.3), a missing revision and the
mode read without a pathspec (§5.4), hooks turned off during status, the vanished-file retry and the
gone upstream (§5.5), paging, the header format and the merges history lists (§5.6), the record
rule, conflicts, unreadable files and history before a rename is committed (§5.8), the approval
merge in §6.2, and rulings 11–21 (§9). Why: the build's reviews and the Windows gate showed where
the approved text and git's behaviour differ.
Source: `docs/specs/00-umbrella.md` §3 P46, umbrella version 8. Version 5 (#33) split the old
"Command layer and git" into P06, P46 Git read and P47 Git write. Scope and acceptance criteria
come from there; this spec decides how. It starts from P05's interfaces (P05 spec §5.2, §6) and
from what P06's brainstorming carried forward (P06 spec §11).

## 1. Goal

After P46, the editor, the CLI and the review can read git through `theos-vcs` and see it by
record:

- `Git::find` refuses a missing or too-old git with a message that says what to install.
- **Status:** what is staged, what is not, and what is new, with renames paired before they are
  staged. Grouped by record in `theos-studio`.
- **History:** the commits of one record, across renames of its file and id (`git log --follow`).
- **Diffs:** old and new per front matter field, and a word-level diff of the prose, for any two
  versions of a record: HEAD, the index, the working tree, a commit or a branch.
- **Process trees:** a long git read can be cancelled, and cancelling stops git and every process
  it started.

The diff itself lives in `theos-content` and needs no git, so P07 can show a proposal's word diff
(CE-25.2) and P12 can compare an unsaved edit with the same code.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-21 Git read**
- CE-21.1 A renamed record keeps its history.
- CE-21.2 Field diff shows old and new per field; prose diff is word-level.
- CE-21.3 A missing or too-old git gives a clear message.

## 3. Scope

**In:**
- in `theos-vcs`:
  - the minimum git version, checked in `Git::find` (CE-21.3), with `GitError::TooOld`;
  - a cancellable run that stops the whole process tree, with `GitError::Cancelled`;
  - the reads: status (§5.5), history (§5.6), a file's bytes at a version (§5.4);
  - one place for the options that make every read independent of the user's git settings;
- in `theos-content`: the word diff, the field diff and the record diff (§5.7);
- in `theos-studio`: the changes by record, a record's diff and a record's history (§5.8), on the
  path mapping that P05's staged check built.

**Out:**
- staging, unstaging, discarding, committing, pulling, pushing, branches, merges (P47; P07 for
  agent branches);
- a command line for the reads (Jann, §4.1); P07 adds `theos review show`;
- the Changes screen and its "Who wrote what" view (P13);
- approval trailers on merge commits shown beside the commits they merged (P07, P13; §6.2);
- the diff of binary files and of images (a binary file shows as changed, nothing more).

## 4. Decisions

### 4.1 Decided by Jann (27 Nov 2026)

- **Minimum git: 2.38.** One minimum for every phase. P07's check of a merged result uses
  `merge-tree --write-tree`, which arrived in 2.38. macOS 13's command-line tools ship 2.39;
  Ubuntu 22.04 ships 2.34, so its users install a newer git.
- **Words are runs of non-space characters,** as `git diff --word-diff
  --word-diff-regex='[^[:space:]]+'`. A re-wrapped paragraph shows no change; "sky." → "sky,"
  shows the whole word changed.
- **Unstaged renames are paired.** A record renamed on disk shows as one renamed entry before it
  is staged (§5.5), so P47 can stage a record by both of its paths.
- **Library only.** No `theos status`, `theos log` or `theos diff` in this phase. P07's review and
  P13's Changes screen are the first users; the ACs are proven by library tests through real git.

### 4.2 Probes (27 Nov 2026, Windows 11, git 2.56.1.windows.1)

- **Status with a temporary index.** A record renamed on disk with its `id:` rewritten
  (`mother-ilse` → `mother-ilsa`) shows in `status --porcelain=v2 -z` as `1 .D` plus `? `. With a
  copy of the index in `GIT_INDEX_FILE` and `git add -N` of the untracked file, the same status
  shows `2 .R … R84 mother-ilsa.md` + `mother-ilse.md`. The real index stays unchanged
  (`ls-files -s` before and after). A file staged and edited again shows `1 MM` in both.
- **History.** `git log --follow -z --name-status --format=<fields separated by %x1f>` over a
  record renamed with its id rewritten and one inbound link updated (P06's rename) lists all three
  commits, the rename as `R083 old new`. `%(trailers:unfold,separator=…)` gives `Approved-by:
  Jann` from a commit's footer.
- **The rename threshold.** A short truth (9 lines) renamed `T-012` → `T-013` in one commit that
  also rewrites its title and its one body line: `git log --follow` with git's default threshold
  (50 %) loses the history (`A T-013.md` only); with `-M30%` it keeps it (`R046`). §9 R1 decides.
- **Process trees.** P06's probe (12 Nov): `Child::kill` on git, run through an alias that starts
  `sh` and `sleep`, left all three running. The plan's re-probe (27 Nov, toolchain 1.99.1), with
  that alias writing a marker file after `sleep 2`; three ways stop the whole tree (no marker
  appears):

  | Way (Windows) | `git --version`, start to exit | Stopping the tree |
  |---|---|---|
  | `std::process::Command`, no tree handling | 8–10 ms | – (the tree lives on) |
  | `process-wrap` 10.0.1 `JobObject` | 80–84 ms: the spawn alone takes 75 ms, because the process starts suspended and is resumed by scanning every thread of the system (Toolhelp snapshot) | yes |
  | `win32job` 2.0.3: a job with kill-on-close, git assigned right after `spawn` | 9–11 ms | 1.6 ms (the job is closed) |
  | `taskkill /T /F /PID` on cancel only | 8–10 ms | 380 ms; misses a process whose parent already exited |

  Jann chose `win32job` (§9 R2). On Unix, std's `CommandExt::process_group(0)` starts git in its
  own group, and `nix`'s `killpg` (nix 0.31 is already in `Cargo.lock`) stops it.
- From P06 §11: `similar`'s `capture_diff_slices` over whitespace-free word tokens gives exactly
  `git diff --word-diff --word-diff-regex='[^[:space:]]+'`; `from_words` and
  `from_unicode_words` count whitespace as tokens, so a re-wrap shows as changes.

### 4.3 Approaches considered

| Part | Chosen | Not chosen, and why |
|---|---|---|
| Git access | The git command line through P05's runner (G6) | `gix` or `git2`: G6 allows only the command line; gix pulled in 166 crates for P05 |
| Word diff | `similar` (already a workspace dependency) over whitespace-free tokens | `similar`'s word tokenisers (whitespace as tokens); `dissimilar` (character-level, not word-level); `git diff --word-diff` (needs both texts in git; P07 and P12 diff texts that are not) |
| Field diff | Compare P02's parsed front matter by value | A line diff of the front matter (shows quoting and list style as changes, not old and new per field) |
| Unstaged renames | `git add -N` on a temporary copy of the index, then git's own rename detection | Pairing deleted and new records in theos (re-implements git's similarity); showing them unpaired (Jann chose pairing) |
| History across renames | `git log --follow` | theos's own walk over rename pairs (re-implements `--follow`); a rename trailer written by P47's commit (misses renames made outside the editor, and the journal that knows renames lives in memory only) |
| Process trees | Windows: `win32job` (a job with kill-on-close); Unix: std `process_group(0)` and `nix::sys::signal::killpg` | `Child::kill` (leaves grandchildren running, probe); `process-wrap` (+70 ms per git start on Windows, probe); `taskkill`/`kill` on cancel (380 ms, misses orphans); own Win32 job code (`unsafe`, forbidden by G3) |
| Version parsing | A small parser of `git version X.Y.Z…` | `semver` (rejects `2.56.1.windows.1` and `2.39.3 (Apple Git-146)`) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Crate | Tool or crate | Custom code, and why |
|---|---|---|---|
| Version check | `theos-vcs` | `git --version` | Parsing the leading `X.Y.Z` of git's answer: no crate reads git's version string (§4.3) |
| Cancellable run, process trees | `theos-vcs` | `win32job` (Windows), std `process_group` and `nix` `killpg` with the `signal` feature (Unix) | The wait loop that watches the cancel flag |
| Read options | `theos-vcs` | git's `-c` and its flags | One list (§5.3) |
| Status | `theos-vcs` | `ls-files`, `add -N`, `status --porcelain=v2 -z` | Parsing git's `-z` porcelain v2 lines, as P05 parses `-z` name-status and `ls-files -s` |
| History | `theos-vcs` | `log --follow --name-status -z` | Parsing the format §5.6 asks for |
| Bytes at a version | `theos-vcs` | `rev-parse`, P05's `blobs` (`cat-file --batch`) | – |
| Word diff | `theos-content` | `similar::capture_diff_slices_deadline` | Splitting into words and mapping token ranges back to byte spans |
| Field diff, record diff | `theos-content` | P02's `Record`, `FrontMatter`, `RawValue` | Comparing fields by value |
| Changes, diffs and history by record | `theos-studio` | P03's index, P05's path mapping | Grouping by record |
| Temporary index file | `theos-vcs` | `tempfile` | – |
| Logging | `theos-vcs` | `tracing` | – |

`theos-vcs` still depends on no other crate of this workspace.

### 5.2 The version check (CE-21.3)

```rust
/// A git version: the first three numbers of `git --version`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GitVersion { pub major: u32, pub minor: u32, pub patch: u32 }

impl GitVersion {
    /// The oldest git theos runs with.
    pub const MINIMUM: GitVersion = GitVersion { major: 2, minor: 38, patch: 0 };
    /// Reads the answer of `git --version`.
    pub fn parse(answer: &str) -> Option<GitVersion>;
}

impl Git {
    /// Finds `git` on the PATH, as before, then runs `git --version` once and refuses a git
    /// older than `GitVersion::MINIMUM`.
    pub fn find() -> Result<Git, GitError>;
    /// The version `find` read.
    pub fn version(&self) -> GitVersion;
}
```

- `parse` takes the text after `git version `, then the leading numbers separated by dots, at most
  three; a missing third number is 0. Accepted shapes: `2.39.3 (Apple Git-146)`,
  `2.56.1.windows.1`, `2.38.0.rc1` (counts as 2.38.0), `2.44.0.123.gabcdef`, `2.40` (2.40.0).
  Anything else is `GitError::Output` with git's answer in the message.
- `find` keeps P05's order: search the PATH, resolve the git environment (P05 §5.2), then the
  version check. The check runs in the current folder with the runner's environment; it needs no
  repository.
- `GitError` gets two variants:

  ```rust
  /// The git found is older than theos needs.
  TooOld { program: PathBuf, found: GitVersion, required: GitVersion },
  /// The caller cancelled the command; git and every process it started were stopped.
  Cancelled { args: Vec<String> },
  ```

- **The messages** (CE-21.3), as `report` prints them and the editor will show them:
  - `NotFound`: "git was not found on the PATH. theos needs git 2.38 or newer: install it from
    https://git-scm.com/downloads, then open a new terminal or restart the editor."
  - `TooOld`: "git 2.34.1 (/usr/bin/git) is too old. theos needs git 2.38 or newer: update it
    from https://git-scm.com/downloads."

  Both name the minimum from `GitVersion::MINIMUM`, never a second copy of the number.
- Every caller of `Git::find` gets the check: `theos check --staged`, `theos hook install`, and
  later the editor and `theos-mcp`.

### 5.3 Running git: cancel, process trees and read options

```rust
/// A flag that cancels the git commands that watch it. Clones share the flag.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);
impl Cancel {
    pub fn new() -> Cancel;
    pub fn cancel(&self);
    pub fn is_cancelled(&self) -> bool;
}

impl Git {
    /// As P05's `run`, and stops git with every process it started when `cancel` is set.
    pub fn run_cancellable(&self, args: &[&OsStr], cwd: &Path, stdin: Option<&[u8]>,
                           env: &[(&str, &str)], cancel: &Cancel) -> Result<Vec<u8>, GitError>;
}
```

- **One spawn path.** `run` keeps its signature (P05 §6) and calls `run_cancellable` with a flag
  nobody sets, so every git command, the hook's included, runs the same way.
- **Process trees.** git is started with `std::process::Command`.
  - Windows: a `win32job::Job` with kill-on-close is made for the run, and git is assigned to it
    right after `spawn` returns, before anything else; every process git starts afterwards joins
    the job. On cancel the job is closed, which ends every process in it. Windows refuses to
    assign a process that has already exited, and on a busy machine a fast git (`git --version`, a
    git that fails at once) can exit before the assignment. So a refused assignment first gives
    git up to 50 ms (checked every millisecond) to finish exiting: a git that has exited keeps its
    result and runs without a job, as it has no tree left. Only a git still running after that is
    killed, and the run is `GitError::Io`.
  - Unix: git starts in its own process group (`CommandExt::process_group(0)`); on cancel the group
    gets `SIGKILL` through `nix::sys::signal::killpg`.

  Then git is waited for, and the run returns `Cancelled`. This is the per-OS code of the phase
  (§5.10).
- **The wait.** stdout and stderr are read on their own threads, stdin is written on a third (as
  in P05). A waiter thread waits for git and hands its exit over a channel; the calling thread
  checks the flag every 10 ms. A cancel is noticed within 50 ms, and a command that finishes in a
  few milliseconds is not slowed by more than a millisecond or two. When a cancel and git's exit
  meet, the run returns `Cancelled` (P47 decides whether a finished commit or push reports its
  result instead).
- A flag set before the run starts returns `Cancelled` without starting git.
- **Read options.** Every read builds its arguments from one list, so a user's git settings
  cannot change what theos parses:
  - `--no-optional-locks` (P05) and `--literal-pathspecs`, so a path with `*`, `?`, `[` or `:` is
    a path, never a pattern;
  - `-c color.ui=never`, `-c log.showSignature=false`, `-c core.quotePath=false`,
    `-c i18n.logOutputEncoding=UTF-8`, `-c log.showRoot=true` (with `log.showRoot=false` a history
    loses its root commit);
  - the format flags each read needs: `-z`, `--porcelain=v2`, `--untracked-files=all`,
    `--find-renames`, `--no-ext-diff`, `--no-textconv`, `--format=…`, `--end-of-options` before
    a revision.

  A test runs every read with a user configuration that sets the opposite of each (§5.10).
- **Logging.** Each git command logs its arguments, its folder and its duration at `debug`,
  through `tracing`. A failed git (`GitError::Failed`) is logged at `debug` too, because many are
  expected (a `rev-parse --verify --quiet` miss, an unset setting); a git that cannot be started or
  whose pipes fail is logged at `warn`.

### 5.4 A file at a version

```rust
/// Where a file's bytes come from.
pub enum Version<'a> {
    /// The commit `HEAD` names.
    Head,
    /// The index, stage 0.
    Index,
    /// Any revision git accepts: a commit id, a branch such as `agent/s-04`.
    Rev(&'a str),
    /// The file on disk.
    WorkTree,
}

/// What a version holds at a path.
pub enum Content { Absent, File(Vec<u8>), NotAFile }   // NotAFile: a link, a submodule, a folder

pub fn read(git: &Git, top: &Path, version: Version<'_>, path: &[u8]) -> Result<Content, GitError>;
```

- `Head`, `Index` and `Rev` resolve the path's object with `git rev-parse --verify --quiet
  --end-of-options <rev>:<path>` (`:0:<path>` for the index, so a path such as `1:notes.md` is
  never read as a merge stage). The mode comes from `ls-tree -z --end-of-options <rev>:<folder>`
  (`<rev>^{tree}` at the top) with the entry matched by its exact name, without a pathspec: on
  Windows git reads a pathspec such as `1:notes.md` as a drive. For the index the mode comes from
  `ls-files -s`. The bytes come from P05's `blobs`. A path the version does not hold is `Absent`;
  a missing `HEAD` (before the first commit) reads every path as `Absent`. A revision that does not
  exist is an error with git's message (`GitError::Failed`), not `Absent`.
- An intent-to-add entry in the index reads as `Absent`: a commit does not record it (P05 §5.2).
- `WorkTree` reads the file under `top` with `std::fs`; a link or a folder is `NotAFile`; a file
  that vanishes between its metadata and its read is `Absent`.
- A path that is not UTF-8 is `GitError::Output` naming the command and the path; theos reads only
  UTF-8 paths, which is what git stores.
- Paths are git's bytes, relative to the top, with `/`. Text normalisation (CRLF, P02) is the
  diff's job, not the read's.

### 5.5 Status

```rust
pub struct Status { pub branch: Branch, pub entries: Vec<Entry> }

pub struct Branch {
    /// The branch name; `None` on a detached HEAD.
    pub head: Option<String>,
    /// The commit; `None` before the first commit.
    pub commit: Option<String>,
    /// The upstream and how far the branch is ahead and behind it, when there is one.
    pub upstream: Option<Upstream>,
}
pub struct Upstream { pub name: String, pub ahead: u32, pub behind: u32 }

pub enum Entry {
    /// A tracked path changed in the index, in the working tree, or both (`1` lines).
    Changed { path: Vec<u8>, staged: Change, unstaged: Change },
    /// A rename (or copy) git paired, in the index or in the working tree (`2` lines).
    Renamed { from: Vec<u8>, to: Vec<u8>, staged: Change, unstaged: Change, score: u8 },
    /// A path with a merge conflict (`u` lines); `P47` and P13 hand it to the merge tool.
    Conflicted { path: Vec<u8>, conflict: Conflict },   // DD, AU, UD, UA, DU, AA, UU
    /// A file git does not track and does not ignore.
    Untracked { path: Vec<u8> },
}
pub enum Change { Unchanged, Modified, TypeChanged, Added, Deleted, Renamed, Copied }

pub fn status(git: &Git, top: &Path, cancel: &Cancel) -> Result<Status, GitError>;
```

`status` runs, in the top folder:

1. `git ls-files --others --exclude-standard -z` gives the untracked files (git's own excludes:
   `.gitignore`, `.git/info/exclude`, `core.excludesFile`).
2. With no untracked files, step 4 runs on the real index; steps 3 and 5 are skipped.
3. Otherwise the index file (`git rev-parse --git-path index`, which honours a `GIT_INDEX_FILE`)
   is copied into a temporary folder (`tempfile::TempDir`: on Windows git cannot rename its lock
   file over an open temporary file); a repository without an index file copies nothing (git
   reads a missing index as empty). With `GIT_INDEX_FILE` set to the copy, `git add
   --intent-to-add --pathspec-from-file=- --pathspec-file-nul` takes the untracked paths on stdin,
   so no command line grows with them (Windows limits it to 32 767 characters). The real index is
   never written; the folder is removed when `status` returns, also on an error or a cancel.
   - The `add` writes the copy whole (`-c core.splitIndex=false`, so no `sharedindex` file appears
     in `.git`) and runs no hook: `-c core.hooksPath=<the temporary folder>` turns off hook files,
     and `-c hook.<name>.enabled=false` turns off each hook the configuration names (git 2.54+;
     the names come from one `config --get-regexp` read). A configured name that `-c` cannot
     express (not UTF-8, or holding `=`) makes `status` fail rather than run the hook.
   - Untracked paths that vanished since step 1 are left out; if the `add` still fails, step 1
     runs once more with a fresh list (an editor's atomic save, a tool's swap file).
4. `git status --porcelain=v2 -z --branch --untracked-files=all --find-renames` with the read
   options of §5.3.
5. Mapping back: a `.A` entry whose path was untracked in step 1 is `Untracked`; a `2` line with
   rename status in the working tree whose new path was untracked is an unstaged rename
   (`Renamed` with `unstaged: Renamed`). An intent-to-add entry the user made (`git add -N`) stays
   `Changed` with `unstaged: Added`, as git shows it.

Paths are bytes in git's spelling, relative to the top. A submodule shows as its path with its
change; theos does not look inside it. An upstream that is gone (no `# branch.ab` line) reads as
`upstream: None`, the same as no upstream: `Upstream` has no "gone" state, and a gone upstream
with ahead and behind 0 would read as "in sync".

### 5.6 History (CE-21.1)

```rust
pub struct Commit {
    pub id: String,
    pub author: Person,          // name and email after `.mailmap`
    pub time: i64,               // author time, seconds since the epoch
    pub offset_minutes: i32,     // the author's time zone
    pub subject: String,
    pub trailers: Vec<Trailer>,  // `Approved-by`, `Co-Authored-By`, … unfolded, in order
    /// How this commit changed the followed file: its path here, and the path before when the
    /// commit renamed it.
    pub change: Change,
    pub path: Vec<u8>,
    pub from: Option<Vec<u8>>,
}
pub struct Person { pub name: String, pub email: String }
pub struct Trailer { pub key: String, pub value: String }
pub struct Page { pub skip: usize, pub max: usize }

pub fn history(git: &Git, top: &Path, path: &[u8], page: Page, cancel: &Cancel)
    -> Result<Vec<Commit>, GitError>;
```

- One command: `git log --follow --find-renames --diff-merges=first-parent --name-status -z
  --date=raw --format=<fields> --max-count=<skip + max> -- <path>`, with the read options of
  §5.3. The first `skip` commits are dropped in theos: git applies `--skip` before `--follow` reads
  the skipped commits, so a rename among them is never followed and the older history is lost.
- The fields are `%H`, `%aN`, `%aE`, `%ad` (`<seconds> <±hhmm>`), the trailers
  (`%(trailers:only,unfold,…)`: `only`, so a non-trailer line in the trailer block, such as
  `git cherry-pick -x`'s, is not read as a trailer) and `%s` last. Each field ends with a NUL, so a
  name, subject or trailer may hold any other byte. A malformed trailer block gives the commit no
  trailers instead of failing the history.
- Newest first. A path never committed, or a repository before its first commit, has an empty
  history (git's "does not have any commits yet" is not an error).
- **The rename threshold** is git's default (§9 R1). A record renamed with P06's `Rename` (the id
  line, the file name and inbound links in other files) keeps its history at any size, because
  only one line of the record changes.
- Merges: under `--follow`, git lists every merge whose result for the file differs from its
  first parent, and shows its change against that parent (`--diff-merges=first-parent`, git
  2.31). So a change made on a branch and merged into `main` is listed twice: the branch's commit
  and the merge. The approval of a merged agent commit is on that merge (§6.2). A rename made on a
  branch merged with `--no-ff` lists the branch's rename commit as `Deleted` at the old path and
  the merge as the rename; the older commits are kept.

### 5.7 The diffs, in `theos-content` (CE-21.2)

A new module `theos_content::diff`. It compares texts; it never runs git.

```rust
/// A word diff: the two texts as a sequence of runs. Each span covers whole words and the
/// whitespace between the words of the same run, never the whitespace before the first or after
/// the last word.
pub struct WordDiff { pub ops: Vec<WordOp> }
pub enum WordOp {
    Equal { old: Span, new: Span },
    Delete { old: Span },
    Insert { new: Span },
}
pub fn words(old: &str, new: &str) -> WordDiff;

pub struct FieldChange { pub key: String, pub delta: FieldDelta }
pub enum FieldDelta {
    Added { new: Field },
    Removed { old: Field },
    /// `items` for a list field: the items added and removed, by value.
    Changed { old: Field, new: Field, items: Option<ItemDelta> },
}
pub struct ItemDelta { pub added: Vec<String>, pub removed: Vec<String> }
pub fn fields(old: &FrontMatter, new: &FrontMatter) -> Vec<FieldChange>;

pub enum RecordDiff {
    Record {
        /// Both sides as P02 read them: LF text, empty for a side that does not exist. Every
        /// span of this diff, the fields' included, points into these texts.
        old: String,
        new: String,
        /// Per field; `Text` when a side's front matter does not parse (a YAML error).
        fields: FieldsDiff,
        /// The body, word by word.
        body: WordDiff,
        /// The bytes differ, but no field value and no word: comments, quoting, list style,
        /// blank lines, a re-wrap, line endings.
        layout_only: bool,
    },
    /// A side is not a record (no `---` lines, a byte-order mark): the whole texts, word by word,
    /// with CRLF read as LF, and the texts the spans point into.
    Text { old: String, new: String, words: WordDiff },
}
pub enum FieldsDiff { Fields(Vec<FieldChange>), Text(WordDiff) }
/// `None` is a side that does not exist: an added record has every field added and every word
/// inserted.
pub fn record(old: Option<&str>, new: Option<&str>) -> RecordDiff;
```

- **Words.** A word is a maximal run of characters that are not git's `[:space:]` (space, `\t`,
  `\n`, `\v`, `\f`, `\r`); a no-break space is part of a word, as in git. The token lists of both
  texts go through `similar::capture_diff_slices_deadline` with Myers and a deadline, so a huge
  rewrite returns a correct but coarser diff instead of hanging the UI. Runs of the same kind next
  to each other are merged. Rendering the new text from `Equal` and `Insert` spans, with the
  whitespace between them taken from the new text, gives the new text exactly; the same holds for
  the old text with `Equal` and `Delete`. Spans are byte spans of the LF texts (P02's `Span`); the
  UI converts them to UTF-16 with P02's `TextIndex`.
- **Fields** are compared by value (`RawValue`): `[a, b]` and the `- a` / `- b` lines are equal,
  `'x'` and `x` are equal, a comment is not a field. Each change keeps both fields, so a caller
  shows the old and the new source text. Fields come in the new record's order; removed fields
  follow, in the old record's order. Every field counts, `id` included.
- **Record.** Both texts are read with P02's `Record::parse` (CRLF becomes LF there). The body is
  the text after the closing `---` line. All spans are byte spans of the whole LF record text,
  never relative to the body, and the diff hands both LF texts back, so a caller never maps spans
  onto a CRLF original.

### 5.8 Changes, diffs and history by record, in `theos-studio`

```rust
pub struct Changes {
    pub branch: Branch,
    /// One entry per record, in the order of the paths.
    pub records: Vec<RecordChange>,
    /// Changed files that are not records: project settings, templates, files outside the
    /// project folder (repository paths).
    pub files: Vec<theos_vcs::status::Entry>,
}
pub struct RecordChange {
    pub id: String,
    pub kind: Option<String>,
    pub title: Option<String>,
    /// The project path now, and the one before when the record was renamed.
    pub path: String,
    pub from: Option<String>,
    /// The same two paths in git's spelling, relative to the top: what `diff` takes.
    pub repo_path: Vec<u8>,
    pub repo_from: Option<Vec<u8>>,
    pub staged: Change,
    pub unstaged: Change,
    pub untracked: bool,
    pub conflict: Option<Conflict>,
}
pub fn changes(workspace: &Workspace, git: &Git, cancel: &Cancel) -> Result<Changes, StudioError>;

/// What a record diff compares.
pub enum Side<'a> {
    Staged,                                  // HEAD → index
    Unstaged,                                // index → working tree
    Commit(&'a theos_vcs::history::Commit),  // the commit's parent (at `from` when renamed) → the commit
    Between(Version<'a>, Version<'a>),       // any two versions, e.g. main → agent/s-04 (P07)
}
pub enum FileDiff { Record(RecordDiff), Text { old: String, new: String, words: WordDiff }, Binary }
/// `path` and `from` are repository paths in git's spelling (`RecordChange::repo_path`, a
/// status entry, a history entry's `commit.path`), so no project path is mapped back.
pub fn diff(workspace: &Workspace, git: &Git, path: &[u8], from: Option<&[u8]>, side: Side<'_>)
    -> Result<FileDiff, StudioError>;

pub struct HistoryEntry {
    pub commit: theos_vcs::history::Commit,
    /// The project path and the id in this commit: they differ from today's after a rename.
    pub path: String,
    pub id: String,
}
pub fn history(workspace: &Workspace, git: &Git, id: &str, page: Page, cancel: &Cancel)
    -> Result<Vec<HistoryEntry>, StudioError>;
```

- **Paths.** Repository paths become project paths with P05's mapping (`project_path` in
  `staged.rs`, under the repository's `Folding`), moved to a module both use. A path outside the
  project folder goes to `files`.
- **Which paths are records.** A path inside the project folder names a record when P03's
  `is_record_path` accepts it and its newest version (the working tree; for a path gone from it,
  the index, else HEAD) parses as a record (`Record::parse`) whose `kind` the registry knows. Its
  id, kind and title come from those bytes, not from P03's index, so `changes` needs no index
  sync, writes no cache and gives the same answer with the in-memory fallback. A path that does
  not parse, names an unknown kind, or cannot be read (a sharing violation on Windows) goes to
  `files`, the last with a warning.
- **Conflicts.** A conflicted path is named from the first of the working tree and HEAD that
  parses as a record, so a conflict inside the front matter (markers there) still lists the
  record by id, with HEAD's kind and title.
- **Cancel.** `changes` checks the flag before each record's reads as well as during the status.
- **The id in a commit** is the file name of the path in that commit without `.md` (the data
  model, §2: the id equals the file name).
- **History by id.** `history` checks the flag first, finds the record's path through P03's index
  (`Index::record`), maps it to git's spelling through one cancellable `ls-files -z` of the index
  under the `Folding` (exact spelling first, then folded), then follows it with
  `theos_vcs::history`.
  - A record renamed but not yet committed (unstaged, or staged) has no commits under its new
    path. Then `history` takes the rename pairing from `theos_vcs::status` and follows the old
    path, so the history shows before the rename is committed.
  - A record that exists only in the working tree, and is no rename, has an empty history. When
    two records share an id, the first path git tracks (in path order) is followed.
- **Diff.** The bytes of both sides come from `theos_vcs::read`. Text that is not UTF-8 is
  `Binary`. A path that names a record diffs with `record`, any other text file with `words`.
- Nothing here takes P06's repo lock or writes the cache: these are reads.

### 5.9 Errors

- `GitError` (non-exhaustive, P05) gets `TooOld` and `Cancelled` (§5.2). Parse failures of git's
  answers are `GitError::Output` with the command and what is wrong.
- `StudioError` gets a variant for git errors if it has none, with the `GitError` as its source,
  so `report` prints each cause once (P05).
- No `unwrap` or `expect` outside tests.

### 5.10 Tests and fixtures

- **Git in tests** as in P05 §5.13: the real `git`, each repository isolated with
  `GIT_CONFIG_GLOBAL` on an empty file and `GIT_CONFIG_NOSYSTEM=1`, identities by `-c`.
- **Hostile user settings:** one test configures, in the test repository, `color.ui=always`,
  `log.showSignature=true`, `core.quotePath=true`, `status.renames=false`,
  `status.showUntrackedFiles=no`, `diff.external`, `log.follow=false` and
  `i18n.logOutputEncoding=ISO-8859-1`, then runs status, history and read and expects the same
  answers as without them.
- **Records** are copies of the data model's examples (§14) and of P02's fixtures. Synthetic ids
  and text say so in the fixture.
- **The version check** needs a git that answers with an old version. A test puts a stand-in
  program named `git` first on the PATH of a `theos` child process. The builder picks the
  stand-in (a script per OS, or a small helper program built for the tests); it must run on all
  three platforms.
- **Per OS:** the process-tree code is the only code that differs per OS. Its tests carry
  `#[cfg(windows)]` and `#[cfg(unix)]`; the PR gets the labels `windows` and `macos`.
- **Cancel test:** git runs an alias `!sleep 2 && echo > <marker>` (git's `sh` on every OS); the
  test cancels after the start and checks that `Cancelled` comes back within the bound of §5.3
  and that the marker never appears, so no process of the tree lived on.

### 5.11 How each AC is proven

| AC | Tests |
|---|---|
| CE-21.1 | **Through real git, in `theos-studio`:**<br>- a record renamed with P06's `Rename` and committed: its history by the new id lists the commits before the rename, with the old path and id, newest first;<br>- the same for the smallest record of the data model's examples;<br>- a rename by `git mv` plus a hand edit of `id:`;<br>- a record moved to another folder (another floor);<br>- two renames in a row (A → B → C).<br>**In `theos-vcs`:** `history` over a rename, paging with `skip` and `max`, a path never committed, a repository before its first commit, trailers. |
| CE-21.2 | **Field diff:** a changed scalar shows the old and the new field; added and removed fields; a list field with items added and removed; quoting or list style alone is `layout_only`; a YAML error on one side gives the word diff of the front matter.<br>**Prose diff:** one changed word in a paragraph is one `Delete` and one `Insert` of that word; a re-wrapped paragraph has no change; CRLF against LF has no change; the reconstruction property of §5.7 holds (`proptest`); an oracle test compares the words added and removed with `git diff --no-index --word-diff=porcelain --word-diff-regex='[^[:space:]]+'` on fixture pairs whose alignment is unique.<br>**Through git, in `theos-studio`:** `diff` of a staged change, an unstaged change and a commit, each showing fields and words. |
| CE-21.3 | **Binary tests of `theos`:** with no `git` on the PATH, `theos check --staged` exits 2 and prints the `NotFound` message; with a stand-in git that answers `git version 2.37.9`, it prints the `TooOld` message with that version, the program's path and 2.38. **Unit tests:** `GitVersion::parse` on the shapes of §5.2 and on answers it refuses; the comparison at 2.37.99, 2.38.0 and 2.38.0.rc1. |

Status (§5.5) and cancel (§5.3) are not ACs of their own; they get behaviour tests: an unstaged
rename paired, the real index unchanged after `status`, the temporary index removed after an
error and after a cancel, a path with a space, `*` and non-ASCII characters, a staged rename with
an unstaged edit, a conflict, a detached HEAD, an upstream ahead and behind, and more untracked
files than fit on one Windows command line.

## 6. Interfaces

### 6.1 From P05 and P06

- `Git::find()`, fallible from day one, gets the version check; `GitError` gets `TooOld` and
  `Cancelled`; `run` keeps its signature; reads pass `--no-optional-locks` (P05 §6).
- `staged::read` and `blobs` (P05 §5.2), and the path mapping of P05's staged check, which moves
  to a shared module.
- P06's `Rename` makes the renames CE-21.1's tests follow.

### 6.2 For other phases

| Phase | Uses |
|---|---|
| P47 | `Cancel` and `run_cancellable` for pull and push; `status` and `Changes` to stage a record by its paths (both paths of a rename); `read` for discarding. Adds its own `GitError` variants |
| P07 | `theos_content::diff::record` and `words` for `theos review show` (CE-25.2), with `Side::Between` for an agent branch against `main`. The `Approved-by:` trailer sits on the approval's merge commit, which `history` lists when the merge changed the file against `main` (§5.6), next to the agent's own commit |
| P12 | `theos_content::diff` to compare an unsaved edit with the saved record |
| P13 | `changes`, `diff` and `history` for the Changes screen (`docs/ui/screens/08-git.html`); `Branch.upstream` for ahead and behind; `Conflicted` entries for CE-36.3; CE-36.2 is `history` |
| P11, P48 | `Git::find` and its messages when they start git |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| A short record renamed and rewritten in the same commit | Its history starts at the rename (probe §4.2) | P06's `Rename` changes one line of the record, so a rename in the editor keeps history; a rename with heavy edits in one commit is the rare rest (§9 R1). For Jann |
| Users on Ubuntu 22.04 (git 2.34) | theos refuses to start git | The message says what to install (CE-21.3). Jann's choice (§4.1) |
| `split-index` or a sparse index in the copied index | The temporary index cannot be read, or reads differently | A test with `core.splitIndex=true`. A failure is a `GitError`, never a wrong answer. Sparse checkouts are not tested (content projects do not use them) |
| User settings that change git's output | Parsing fails or misreads | The read options of §5.3 and the hostile-settings test |
| A job object inside a job object (theos in a hook, run by an IDE that uses jobs) | Creating the job fails on old Windows | Windows 8 and later nest jobs; G2's Windows is 11 |
| A process git starts before it is assigned to the job (Windows) | That process survives a cancel | git loads for milliseconds before it starts anything, and the assignment follows `spawn` at once; the cancel test proves the tree stops |
| git exits before it is assigned to the job (Windows, a busy machine) | Windows refuses the assignment | A git that has exited, or exits within 50 ms, keeps its result and runs without a job (§5.3); found by the gate's instrumented run. A git still exiting after 50 ms fails with `GitError::Io` |
| git in its own process group (Unix) | Ctrl+C in a terminal reaches theos, not git | theos's reads end in milliseconds; P47 decides for pull and push, which can wait on the network |
| Myers and git's xdiff choose different alignments when several are minimal | The editor marks other words than `git diff --word-diff` | Both are minimal; the oracle test uses pairs with one alignment |
| A huge record or a rewrite of all of it | A slow diff | The deadline of §5.7 |
| `log --follow` follows only one path and simplifies merges | A history through a merge that renamed the file on one side may lose the older part | Tests cover renames on a branch merged into `main`; a loss there is a `--follow` limit, listed for Jann if the build finds one |

## 8. Review Focus — inputs most likely to break it

- **Paths:** spaces, `*`, `?`, `[`, `:`, a leading `-`, non-ASCII (NFC and NFD on macOS), a
  case-only rename on a case-insensitive disk, a project in a subfolder with changes outside it.
- **Status:** an unstaged rename with the id rewritten; a staged rename with an unstaged edit;
  a file staged then deleted on disk; a user's own `git add -N`; untracked files inside an
  untracked folder; ignored files; no index file; no commit yet; a detached HEAD; a conflict of
  each kind; an upstream that is gone.
- **The temporary index:** removed after success, error and cancel; never the real index; a hook's
  `GIT_INDEX_FILE` (status run inside `commit -a`).
- **History:** a subject or trailer with `%`, quotes, tabs, separators and non-ASCII; a trailer
  folded over lines; a record renamed twice; a rename on a branch then merged; the first commit;
  `skip` past the end.
- **Diffs:** empty texts; a record without a body; whitespace-only changes; a word moved; a no-break
  space; CRLF on one side; a YAML error on one side; a field whose value changes type (scalar to
  list); a duplicate key; non-UTF-8 bytes.
- **Version:** every shape of §5.2; a `git` that prints nothing or exits with an error.
- **Cancel:** a cancel before the start, during the stdin write, after the exit; a git that
  writes more than a pipe holds.

## 9. Rulings (Claude's calls)

1. Ruling: history follows renames with git's default threshold (50 %), not a lower one — records
   share boilerplate lines (`kind: truth`, `status: draft`, `target: hint`), so a lower threshold
   would join the histories of two unrelated short records deleted and created in one commit, and
   the editor's own rename changes one line of the record — cost if wrong: a short record renamed
   and rewritten in one commit loses its older history in the editor, as it does in `git log
   --follow`; a lower threshold is then a one-line change.
2. Ruling (Jann, 27 Nov 2026): `win32job` 2.0.3 on Windows and a process group with `nix`'s
   `killpg` on Unix, not `process-wrap` — `process-wrap` adds about 70 ms to every git start on
   Windows (§4.2), and 2.0.3 is the version probed (2.1.0 is 17 days old) — cost if wrong: a
   process started in the microseconds before the assignment survives a cancel.
3. Ruling: one spawn path for every git command, `run` included — two paths would test the
   process-tree code only on the reads; with `win32job` a start costs no measurable time — cost if
   wrong: the hook also starts git in a job object, which a policy that forbids nested jobs would
   break.
4. Ruling: fields are compared by value, and a byte change without a field or word change is
   `layout_only` — CE-21.2 asks for old and new per field, which is the value; a reformat is not a
   content change — cost if wrong: a comment edit is visible only as "layout only".
5. Ruling: a word is a run of characters outside git's ASCII `[:space:]`, so a no-break space
   joins words — it makes the diff equal to `git diff --word-diff` with the regex Jann chose —
   cost if wrong: French typography's no-break spaces join a word to its punctuation.
6. Ruling: `.mailmap` names authors (`%aN`, `%aE`) — it is how a repository corrects its own
   authors — cost if wrong: a history shows the corrected name instead of the recorded one.
7. Ruling: the temporary index is skipped when nothing is untracked — the pairing needs untracked
   files, and the copy costs a write of the index — cost if wrong: none; both paths are tested.
8. Ruling: a merge that history lists shows its change against its first parent — without a
   `--diff-merges` choice git prints no change for a merge, and the first parent is the branch the
   change came into — cost if wrong: a resolved conflict shows against `main` only, not against
   the merged branch.
9. Ruling: `changes` decides which paths are records from their bytes, not from P03's index — a
   read must not sync or write the cache, and the in-memory fallback must give the same answer —
   cost if wrong: every changed path is parsed once per status (milliseconds for a few files).
10. Ruling: `diff` takes repository paths in git's spelling — status and history already hold
    them, and mapping a project path back would need git's folding a second time — cost if
    wrong: a caller with only a project path looks up its `RecordChange` first.

Rulings 11–21 were made during the build (PR #62) and are part of the spec as built.

11. Ruling: paging uses `--max-count=skip+max` and drops the first `skip` commits in theos — git
    applies `--skip` before `--follow` sees the skipped renames — cost if wrong: deep pages
    re-walk the newer commits.
12. Ruling: history lists every merge git shows under `--follow --diff-merges=first-parent`, so a
    merged branch change appears twice — filtering merges in theos would re-implement history
    simplification and break paging, and the approval merge's trailer is then in the list — cost
    if wrong: P13 folds the duplicates.
13. Ruling: a gone upstream reads as `upstream: None` (§5.5) — the type cannot say "gone" — cost
    if wrong: P13 cannot show "upstream gone" until `Upstream` gains a state.
14. Ruling: the index copy lives in a temporary folder and is written whole
    (`core.splitIndex=false`) — Windows cannot rename git's lock file over an open temporary file,
    and a split index would write a `sharedindex` file into `.git` — cost if wrong: the copy of a
    split index is written whole.
15. Ruling: `status` turns off hook files and every configured hook, and fails closed on a
    configured hook name `-c` cannot express — status must never run repository code — cost if
    wrong: `status` errors in such a repository instead of listing it.
16. Ruling: a refused job assignment gives git up to 50 ms to finish exiting before it counts as
    running (§5.3) — Windows refuses exited and exiting processes — cost if wrong: a git still
    exiting after 50 ms on a very busy machine fails with `GitError::Io`.
17. Ruling: the id is the file name without `.md`, also in `changes`; a rename from outside the
    project folder has `from: None` with `repo_from` set — the data model's id rule — cost if
    wrong: a file whose `id:` disagrees with its name shows under its file name.
18. Ruling: a conflicted record is named from the working tree, else HEAD (§5.8) — P13 lists pull
    conflicts by record (CE-36.3) — cost if wrong: its title shows HEAD's value.
19. Ruling: an unreadable path goes to `files` with a warning instead of failing `changes` — one
    locked file must not hide the rest — cost if wrong: a real IO fault shows as an unclassified
    file.
20. Ruling: the old side of a diff is read at `from`, and at `path` when the old version holds
    nothing at `from`; `Side::Commit` uses the commit's own paths; a side that is no regular file
    is `Binary` — the cases a staged rename with an unstaged edit, a followed history and a link or
    submodule need — cost if wrong: a caller's paths are ignored for `Side::Commit`.
21. Ruling: a non-UTF-8 author, email, subject or trailer is read lossily; the commit id must be
    UTF-8 — git stores UTF-8 by default and the read options force it — cost if wrong: a
    legacy-encoded name shows replacement characters.

## 10. Left to the builder

- Module names inside the three crates.
- The separators of the history format and the parser of its output.
- The waiting loop of the cancellable run, within §5.3.
- The stand-in git of the version test, within §5.10.
- Whether `Changes` orders records by path or by id.
- The deadline of the word diff.
