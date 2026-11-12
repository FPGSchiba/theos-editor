# P05 · CLI and hook — phase spec

Status: waiting for Jann's approval · 12 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P05, version 6 (the P05 split, docs PR #34). Scope and
acceptance criteria come from there; this spec decides how.

## 1. Goal

After P05, `theos` is the command line of a content project:
- `theos check` prints every finding of the project;
- `theos ledger` prints the reveal ledger;
- `theos new` creates a record where its kind belongs;
- `theos schema` describes the kinds and the rules;
- `theos hook install` writes a git pre-commit hook.

Every one of them prints text by default and one JSON document with `--json`.

The hook runs `theos check --staged`. That command checks the snapshot that the commit would
record, not the working tree, and refuses the commit when it holds an error. A never-confirm
violation is always an error (G8).

The work lands in three crates, so the editor, MCP and the editor's own commit use the same code
as the CLI:
- `theos-vcs` gets the git runner and the reader of the staged snapshot;
- `theos-studio` gets the workspace (project, kinds and index, opened once), the checks and the
  JSON views;
- `theos` keeps only argument parsing and printing.

Later phases build on it (§6):
- P06 builds the command layer on the workspace;
- P46 adds git's version check and the git reads to the runner;
- P47's commit calls the same staged check as the hook, so both block on the same findings;
- P48 serves the same views over MCP;
- P49 releases the binaries, adds `--version`, and builds the check action whose problem matcher
  reads `theos check`'s lines.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-17 CLI and hook**
- CE-17.1 `theos check | ledger | new | schema | hook install` with `--json` output.
- CE-17.2 The hook refuses a commit with a never-confirm violation and checks the staged snapshot.

## 3. Scope

**In:**
- the commands `check` (with `--staged`), `ledger`, `new`, `schema` and `hook install`, and
  `--json` for each, `formalism` included;
- finding the project from the current folder;
- exit codes;
- reading the binaries folder from the user's settings for `theos check`;
- in `theos-vcs`: `Git` and `staged`;
- in `theos-studio`: `Workspace`, the checks, the matching of introduced findings (§5.9), the
  views, the error report and the user settings value `binaries_root` with its folder override
  `THEOS_CONFIG_DIR` (§5.10);
- in `theos-content`: a snapshot preview and the list of the paths the cache holds (§5.7), and the
  placement folder as a shared function (§5.9);
- P04's follow-up: `theos formalism` prints a source error twice (§5.12).

**Before the build:** #34 (umbrella version 6) is merged, so `ac-trace` reads P05's criteria,
CE-17.1 and CE-17.2, from `main`.

**Out:**
- the MCP server (P48);
- releases, `--version` with commit, schema and protocol versions, and the check action (P49).
  Until P49, `--version` prints clap's `theos 0.0.0`;
- the command layer, undo and the repo lock (P06);
- git's version check, history, diffs and status (P46);
- the commit itself (P47);
- `theos review` (P07) and `theos gen` (P09);
- JSON Schema export of the kinds;
- generating the content project's templates from the kinds;
- layered settings beyond the one value `binaries_root` (P14).

## 4. Decisions

### 4.1 Decided by Jann (12 Nov 2026)

- **P05 split into three focused phases:** P05 CLI and hook, P48 MCP read, P49 Releases and the
  check action (umbrella version 6, #34). The P49 decisions are recorded in its umbrella entry:
  - a gate PR first;
  - the check action at `actions/theos-check/`;
  - proof by dry runs before the first release, `v0.1.0-rc.1`;
  - build provenance once the repository is public.

### 4.2 Agreed with P06's spec session (12 Nov 2026)

P05 and P06 both check the staged snapshot: the hook here, P47's commit (CE-22.1) there. Jann asked
the two specs to settle who owns it. Agreed:
- **P05 builds** `theos_vcs::Git` and `theos_vcs::staged`, the `Workspace` in `theos-studio`, and
  the one staged check, `theos_studio::check_staged`.
- **P06's phases build the rest:**
  - P46 extends `Git` with the version check (CE-21.3) and the reads;
  - P47's commit calls `check_staged`.

  The hook and the commit block on the same findings.
- **What the check covers:** every finding of the staged snapshot, not only the records the commit
  changes. P06 found the case that decides it. T-012 is staged in full with its target lowered,
  so the working copy equals the index and a preview has nothing to revalidate. The committed
  clue C-031 now overshoots, and a check of only the changed records would let that no-overshoot
  violation (G8) through.
- **No repo lock:** the staged check never takes P06's repo lock. P47's commit holds it while git
  runs the hook.
- **`theos new`** writes its file only after a preview of the new record. It follows Jann's
  ruling for P06's commands: only an error the command introduces rejects it. Errors already there
  come back with it and do not block. Every error on the new record counts as introduced. On any
  other path, an error counts as introduced only when the number of errors of that rule on that
  path grows. P06 matches by path and rule, not by message, because P04's messages change with
  ordinary edits (three-clue's "No clue" becomes "Only one clue"). The matching is built once in
  `theos-studio` (`introduced`) by whichever of P05 and P06 builds first, and the other uses it.
  Whichever merges second moves `theos new` onto P06's create command. Both write with a
  create that never replaces an existing file.
- **Interfaces:** the signatures of §6 are the contract both specs name.
- **Timing:** whichever PR merges second merges `origin/main`.

### 4.3 Probes (12 Nov 2026, toolchain 1.99.1, Windows 11, git 2.56.1)

- **What a pre-commit hook sees.** In a scratch repository, a hook printed the commit's paths
  (`git diff --cached --name-status --no-renames -z`), the paths whose working copy differs from
  the index (`git diff --name-only --no-renames -z`), the untracked files
  (`git ls-files --others --exclude-standard -z`) and the staged bytes (`git cat-file --batch`).
  - **Plain `git commit`:** `GIT_INDEX_FILE=.git/index`; a file staged and then edited again
    shows its staged bytes.
  - **`git commit -a`:** `GIT_INDEX_FILE=<repo>/.git/index.lock`; the snapshot holds every
    tracked change.
  - **`git commit <path>`:** `GIT_INDEX_FILE=<repo>/.git/next-index-<pid>.lock`; the snapshot
    holds only that path's change.
  - In all three, git commands run by the hook use that index without being told: the
    environment carries it.
  - A relative `GIT_INDEX_FILE` also resolved from a subfolder, and `git diff --cached` works
    before the first commit, where it lists every staged file as added.
- **gix 0.90** builds with `index`, `blob-diff`, `status` and `sha1` and pulls in 166 crates.
  The git command line needs none, and G6 allows only the command line.
- Handed to P48 and P49: rmcp 3.9.0 over stdio, release-plz 0.4.3, dist 0.34.0, and the
  vergen-gitcl result. vergen-gitcl reports a stale commit inside a git worktree, so P49 needs
  another way to get the commit.

### 4.4 Approaches considered

| Question | Chosen | Rejected and why |
|---|---|---|
| Where the logic lives | `theos-studio`: workspace, checks and views; `theos` only parses and prints | In the binary: P48's MCP server needs the same checks and views, and architecture §1 asks for one API for the UI, CLI and MCP |
| JSON shapes | View structs with `serde::Serialize` in `theos-studio` | `Serialize` on `theos-content`'s types would tie a public contract to internal types; renaming a field there would silently break `--json` and MCP |
| Reading the staged snapshot | The git command line (G6) | gix: G6 forbids it, and it pulls in 166 crates |
| What the hook checks | Every finding of the staged snapshot (§4.2) | Only changed records and their neighbours: misses a lowered target. Only the errors a commit introduces: needs a second preview of `HEAD` and a second index |
| Where `theos new` takes the text from | The project's `templates/records/<kind>.template.md`, else a skeleton from the kind | Kind only: ignores the templates Jann keeps for Obsidian. Templates only: fails for a project without them, such as the test fixtures |
| How findings reach GitHub's annotations (P49) | P49's problem matcher reads the text lines, so the line format is a contract from now on | A `--format github` output: a second format to keep, where a matcher configures GitHub's own tool (directive 2) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Tool or crate | Why custom where custom |
|---|---|---|
| Arguments | `clap` (derive), already used | |
| JSON | `serde`, `serde_json` | |
| Running git | `std::process::Command`, no shell | G6. `which` resolves the program once (§5.2) |
| Staged entries | `git ls-files -s -z` and `git cat-file --batch`, read by a small reader | git documents both formats. The only Rust parsers are part of gix (G6). The reader splits NUL-separated records and the batch header `<id> <type> <size>` |
| User settings path | `dirs::config_dir()` (dirs 7), `toml` | It gives exactly architecture §13's folders: `%APPDATA%`, `$XDG_CONFIG_HOME`, `~/Library/Application Support` |
| Front-matter values from flags | P02's patcher (`PatchOp`) | |
| Where a new record goes | The placement check's own folder rule, made a shared function | |
| Validating before a write | `Index::preview` | |
| The staged snapshot's findings | A new `Index::snapshot` in `theos-content` (§5.7) | |
| Error text | `std::error::Error::source` and a small report function (§5.12) | std and anyhow have no option to drop a cause that the message already contains |
| The hook script | A POSIX `sh` script | Git runs hooks with `sh` on every OS, Git for Windows included |
| `{{title}}` in templates | Plain replacement of that one placeholder | A template engine would also treat a compile template's own `{{speaker}}` as a variable, and Obsidian's templates use `{{title}}` only (§5.9) |

### 5.2 `theos-vcs`: the git runner and the staged snapshot

`theos-vcs` depends on no other crate of this workspace.

```rust
/// The git program, found once.
pub struct Git { program: PathBuf }

impl Git {
    /// Finds `git` on the PATH. P46 adds the minimum-version check here.
    pub fn find() -> Result<Git, GitError>;
    /// Runs git with `args` in `cwd`, with no shell. Passes `stdin` when given and adds `env` to
    /// the inherited environment (so a hook's `GIT_INDEX_FILE` applies). Returns stdout.
    pub fn run(&self, args: &[&OsStr], cwd: &Path, stdin: Option<&[u8]>,
               env: &[(&str, &str)]) -> Result<Vec<u8>, GitError>;
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    NotFound,                                              // git is not on the PATH
    Failed { args: Vec<String>, code: Option<i32>, stderr: String },   // stderr whole, as text
    Io { source: std::io::Error },
}
```

P46 adds `TooOld { found, required }` and a cancel variant, and a cancellable spawn beside `run`.
The parameter types of `run` are the plan's call, as long as P46 and P47 can add to them without
changing them.

```rust
/// What a commit would record, read through git.
pub struct Staged {
    /// The repository's top folder.
    pub top: PathBuf,
    /// The commit's paths and how each changes, from `diff --cached`.
    pub changed: Vec<StagedPath>,
    /// Every path in the index under the folder read, with its mode.
    pub tracked: Vec<Tracked>,
    /// The bytes of each tracked regular file whose working copy differs from the index.
    pub modified: Vec<(Vec<u8>, Vec<u8>)>,   // (path, staged bytes)
}
pub struct StagedPath { pub path: Vec<u8>, pub status: Status }   // A, M, D, T, …
pub struct Tracked { pub path: Vec<u8>, pub mode: u32 }

pub fn read(git: &Git, folder: &Path) -> Result<Staged, GitError>;
```

`read` runs, in this order:
1. `git rev-parse --show-toplevel` in `folder`. Every later command runs in that top folder,
   where git runs hooks, so a relative `GIT_INDEX_FILE` resolves as git meant it.
2. `git --no-optional-locks diff --cached --name-status --no-renames -z` gives `changed`. Before
   the first commit it lists every staged file as added (probe §4.3).
3. `git --no-optional-locks ls-files -s -z -- <folder>` gives `tracked`: every index entry under
   the folder, with its mode and object id.
4. `git --no-optional-locks diff --name-only --no-renames -z -- <folder>` gives the tracked paths
   whose working copy is modified or deleted. For the regular files among them, one
   `git cat-file --batch` reads the staged bytes by object id, so no path ever goes into the
   batch input. That gives `modified`.

Untracked files need no git call. The cache knows which files the working tree holds, and a path
that the cache holds but `tracked` does not is not in the snapshot (§5.7). git's own exclude
sources (`.git/info/exclude`, `core.excludesFile`, `.gitignore` files above the project) differ
from the index's walk, which reads only the project's own `.gitignore` files (P03). Listing
untracked files through git would let a local-only record that git excludes count as committed.

Every read passes `--no-optional-locks`, so it never refreshes the index or takes
`.git/index.lock` while a commit runs. Paths stay bytes. `theos-studio` turns them into project
paths (§5.7).

### 5.3 `theos-studio`: the workspace

```rust
/// One working tree of a content project: its project, kinds and index, opened once.
pub struct Workspace { project: Project, registry: KindRegistry, index: Index, in_memory: bool }

impl Workspace {
    /// Opens the project at `root`: `Project::open`, `parts::builtin()`, `Index::open`.
    pub fn open(root: &Path) -> Result<Workspace, StudioError>;
    /// The nearest folder at or above `start` that holds a `theos.toml`, opened.
    pub fn find(start: &Path) -> Result<Workspace, StudioError>;
    pub fn project(&self) -> &Project;
    pub fn registry(&self) -> &KindRegistry;
    pub fn index(&self) -> &Index;
    pub fn index_mut(&mut self) -> &mut Index;
    /// True when the shared cache was contended and this workspace reads an in-memory index.
    pub fn is_in_memory(&self) -> bool;
}
```

- **The registry** is `parts::builtin()`. The editor-part registry comes with the first phase
  that registers something per part (P09 or P11), behind the same constructors. P06 adds
  `impl Workspace` blocks in its own modules.
- **One workspace per working tree:** in P07 that is each agent worktree, since P03 plans one
  index per worktree.
- **Contended cache** (P03 §7). Another theos build may keep rebuilding the shared cache until
  `Index::open` or a later write fails with `IndexError::Contended`. This happens with a released
  `theos-mcp` beside a development build of the editor. The workspace then builds an in-memory
  index from the files (`Index::in_memory` and `sync`, under a second for 500 records), and
  `is_in_memory` turns true. The CLI prints a warning that names the cause; the result is the
  same.
- `StudioError` is a `thiserror` enum. Its variants are `Project`, `Registry`, `Index`, `Git`,
  `Io { path, source }`, `NotAProject { start }` (no `theos.toml` at or above the folder) and
  `NotInRepository { folder }`. It is `#[non_exhaustive]`.

### 5.4 The `theos` binary: commands, project, output, exit codes

```text
theos check   [<folder>] [--staged] [--binaries <folder>] [--json]
theos ledger  [<folder>] [--json]
theos new     <kind> [<id>] [--title <text>] [--set <field>=<value>]… [--project <folder>]
              [--json]
theos schema  [<kind>] [--project <folder>] [--json]
theos hook install [<folder>] [--force] [--json]
theos formalism [<folder>] [--json]
```

- **Project:** a `<folder>` given is the project root, as in P04. Without one, the project is the
  nearest folder at or above the current one that holds `theos.toml`, like git and cargo
  (`Workspace::find`). `new` and `schema` take the folder as `--project`, because their
  positionals are a kind and an id.
- **Text:** stdout carries only the result. A finding is one line in P03's `Display` form:
  - `path:line:column: severity rule: message` when it has a position;
  - `path: severity rule: message` when it has none.

  `path` is project-relative with `/`; line and column start at 1, and the column counts Unicode
  scalar values (P02). P49's problem matcher reads exactly these lines. Notes and the summary go
  to stderr, for example `2 errors, 1 warning` and `binaries not checked: no binaries folder
  set`.
- **`--json`:** one JSON document on stdout, pretty-printed and ending in a newline. The shapes
  are in §5.11. Notes stay on stderr, and the summary is part of the document.
- **Exit codes:**
  - 0: the command ran, with no error finding. Warnings are allowed.
  - 1: `check` ran and found at least one error. No other command uses 1.
  - 2: the command could not run: no project, git missing or failing, I/O, an invalid flag
    (clap already uses 2), a refused `new`, or a refused `hook install`.

  P04's `ledger` and `formalism` move their failure code from 1 to 2. A script can then tell "the
  canon has errors" from "theos could not check".
- **Errors:** printed to stderr as `error: <report>` (§5.12).

### 5.5 `theos check`

1. `Workspace::open` or `find` syncs the index with the folder. Unchanged files are not read
   again (P03).
2. Findings: `Index::findings()`, which applies the configured severities. A guardrail is always
   an error.
3. Binaries (P04 §5.13): the binaries folder is `--binaries <folder>` if given, else
   `binaries_root` from the user settings (§5.10). With a folder, `binary_findings(folder)` adds
   its findings, which are warnings. Without one, the note says that binaries were not checked
   and why.
4. Prints in index order, the order of path, line, column and rule. Exits 1 on any error.

`--staged` is §5.7.

### 5.6 `theos ledger`, `theos formalism`, `theos schema`

- **`theos ledger`:** P04's text unchanged; `--json` prints the ledger view (§5.11).
- **`theos formalism`:** unchanged; `--json` prints `{ "written": [paths] }`.
- **`theos schema`** without a kind lists:
  - every kind: name, part, id rule (the rule's `says`) and placement (a folder, or "under its
    `parent`");
  - every rule from `rules::all`: id, default severity, class, and the severity the project
    configures when a project is found. Outside a project the configured column is left out.

  `theos schema <kind>` prints the kind in full: its fields in front-matter order (key, type in
  P03's `Display` form, required), its id rule, its placement, and its body grammar (beat
  attributes with their targets, note targets, verbatim or not). An unknown kind is exit 2,
  listing the known ones. The kinds come from `parts::builtin()`, so `schema` needs no project.
  `--json` prints the same as data (§5.11). It is not a JSON Schema document (§3, Out).

### 5.7 The staged check (CE-17.2)

```rust
pub struct StagedCheck {
    /// Every finding of the staged snapshot, configured; those on `changed` paths first.
    pub findings: Vec<Finding>,
    /// The commit's paths, project-relative.
    pub changed: Vec<String>,
}
impl StagedCheck { pub fn blocks(&self) -> bool }   // any finding with severity error

pub fn check_staged(workspace: &mut Workspace, git: &Git) -> Result<StagedCheck, StudioError>;
```

**Steps:**
1. `staged::read(git, project root)`.
2. **Map paths:** project path = repository path minus the project's place in the repository.
   The project's place is its root relative to `Staged::top`, after both are made absolute with
   `std::fs::canonicalize`. Paths outside the project are dropped. A path that is not UTF-8
   cannot be a record or an input file, so it is dropped too.
3. **Build the changes** relative to the cache. The cache mirrors the working tree once the
   workspace has synced it. A new `Index::paths()` lists every record and input file it holds.
   - For each `modified` path: `Change::Content(path, staged bytes)`. This covers a file staged
     and then edited again, and a file deleted on disk but still staged.
   - For each cached path that is not a regular file in `tracked` (mode 100644 or 100755):
     `Change::Removed(path)`. This covers an untracked or locally excluded record, a staged
     deletion, and a symbolic link or submodule entry at a record path.
   - A tracked path that the cache does not hold and that is not modified gets no change. The
     walk leaves it out (a project `.gitignore` or a hidden file), so for the index it is not a
     record, staged or not.
4. **Snapshot preview:** `Index::snapshot(&changes)` runs. It is one of two additions to
   `theos-content`'s index; `paths()` is the other:
   - it applies the changes in a transaction and rolls it back, as `preview` does;
   - but it returns every finding of the resulting state, configured with the snapshot's own
     `.theos/settings.toml`, instead of only the touched paths.

   It is `preview`'s `transact` with a read of all findings: one function on the shared path,
   no copy. A staged settings change therefore applies to the whole snapshot, also to paths it
   did not touch. A staged lowering of a guardrail is a finding (CE-14.1) and an error, and it
   blocks.
5. **Order:** findings on `changed` paths first, then the rest, each in index order.
6. **Block:** `blocks()` is true for any error. `theos check --staged` prints the findings, says on
   stderr `commit refused: N errors in the staged snapshot` or
   `staged snapshot: no errors, N warnings`, and exits 1 or 0.

**Binaries:** `--staged` checks no binaries. Binary findings are warnings, so they never block,
and the hook stays offline and fast.

**No repo lock:** `check_staged` never takes P06's repo lock (§4.2). It writes only the index
cache: one sync transaction, then the rolled-back snapshot transaction. P47's commit must not
hold an index transaction while git runs the hook (§6).

**What this proves:**
- a never-confirm violation in the snapshot always blocks, wherever it sits;
- a fix that exists only in the working tree does not let a staged violation through;
- a violation that exists only in the working tree does not block a clean snapshot.

### 5.8 `theos hook install`

1. Finds the project as in §5.4 and `Git::find()`, then the repository's top folder
   (`git rev-parse --show-toplevel`). The git commands below run in that top folder, the folder
   git runs hooks in.
2. **Finds the hooks folder** with `git rev-parse --git-path hooks`, resolved against the top
   folder. That honours `core.hooksPath`, and a linked worktree shares the common folder's hooks.
   The folder must lie inside the repository's git folder
   (`git rev-parse --path-format=absolute --git-common-dir`). Otherwise `hook install` exits 2
   and explains why:
   - a global `core.hooksPath` would install the hook into every repository of the user, and its
     commits outside a theos project would all be refused;
   - a tracked hooks folder would commit a path that only fits this machine.
3. Writes `pre-commit` with LF line ends:

   ```sh
   #!/bin/sh
   # theos pre-commit hook, written by `theos hook install` (CE-17.2).
   # It checks the staged snapshot and refuses the commit on any error.
   theos='C:/Users/jann/bin/theos.exe'
   [ -x "$theos" ] || theos=theos
   exec "$theos" check --staged '.'
   ```

   - The path is that of the `theos` that installs the hook (`std::env::current_exe`), with `/`,
     inside single quotes, a `'` written as `'\''`.
   - If the file is gone later (moved, or a new release unpacked elsewhere), the hook falls back
     to `theos` on the PATH.
   - The last argument is the project folder relative to the top folder, with `/`: `'.'` for a
     project at the top, `'content'` for one in `content/`. Git runs the hook in the top folder,
     so the hook needs no search. Because the path is relative, it also fits every linked
     worktree, whose top holds the same layout.
   - On Unix the file gets mode 0755. Git for Windows runs hooks without an executable bit.
4. **An existing `pre-commit`:**
   - with the marker line (`# theos pre-commit hook`) and the same project folder, it is
     replaced: a re-install updates the path;
   - a marked hook for another project folder of the same repository, or any other hook, is left
     alone, and the command exits 2, naming the file. One hook checks one project;
   - with `--force` the existing hook is first renamed to `pre-commit.before-theos`, and the
     message names that file.
5. Prints the hook's path. `--json` prints
   `{ "hook": path, "replaced": bool, "backup": path|null }`.

A folder outside a git repository is exit 2: "`<folder>` is not in a git repository".

### 5.9 `theos new`

```text
theos new npc mother-ilse --set parent=fates-cult --set home=fates-shrine --title "Mother Ilse"
theos new truth --title "…"        # next free number: T-<n+1>
```

1. **Kind:** from the workspace's registry. An unknown kind is exit 2 with the list.
2. **Id:**
   - given: it must fit the kind's id rule (`KindRegistry::id_fits`). An id that exists in any
     case is caught twice: `create_new` fails for the same file, and the preview's
     `index/duplicate-id` error, which folds case (P03), is introduced by the new record;
   - not given, for a numbered kind (`T-`, `B-`, `C-`, `S-` and `Q-`, data model §3): the
     highest number of that prefix in the index plus one, with at least the digits of the widest
     existing id (`T-012` → `T-013`) and at least three;
   - not given, for a slug kind: exit 2, "give an id".
3. **Text:**
   - when `templates/records/<kind>.template.md` exists, its text with every `{{title}}` replaced
     by the id. Obsidian replaces `{{title}}` with the file name, which is the id. Other `{{…}}`
     stay: a compile template's `{{speaker}}` is content;
   - otherwise a skeleton built from the kind: `id`, `kind` and `title` (the id), `status: draft`
     where the kind has `status`, and an empty body.
4. **Flags:** `--title` and each `--set field=value` go through P02's patcher, so they are checked
   as the restricted YAML of the data model, and a list is written `[a, b]`. A field the kind does
   not have is exit 2.
5. **Path:**
   - `Placement::Flat(folder)`: `<folder>/<id>.md`;
   - `Placement::UnderParent(field)`: the folder the placement check wants,
     `<parent's folder>/<parent id>/<id>.md`.
     That rule moves out of the check into one shared `theos-content` function, used by both. A
     parent that does not exist is exit 2.
6. **Preview:** `Index::preview` runs with the new text. `theos_studio::introduced(before,
   after)` compares by P06's rule:
   - before: `findings_for` each affected path, read after the rolled-back preview;
   - after: the preview's findings, with configured severities;
   - every error on the new record is introduced;
   - on any other path, the errors of one rule are introduced when that path has more of them
     after than before.

   An introduced error rejects the command (exit 2): the findings are printed, and no file is
   written. The template's `parent: todo` would dangle on the new record, so it needs
   `--set parent=…`. Standing errors elsewhere and warnings are printed, and the file is written.
7. **Write:** the file is created only if it does not exist (`create_new`), in LF. Then
   `Index::update(&[path])` runs. The command prints the path. `--json` prints
   `{ "path", "id", "findings", "introduced" }`.

The command layer (P06) gets a create command. Whichever PR merges second moves `theos new` onto
it, with the same behaviour (§4.2).

### 5.10 The user setting `binaries_root`

`theos_studio::user_settings::binaries_root() -> Result<Option<PathBuf>, StudioError>`:
- reads `settings.toml` in the user settings folder: `THEOS_CONFIG_DIR` when that environment
  variable is set, else `<dirs::config_dir()>/theos`, the user layer of architecture §13. On
  Windows, `dirs` asks the OS for the folder and ignores `%APPDATA%`. Without the variable, the
  binary tests would read the settings of the machine that runs them;
- returns its top-level string `binaries_root`;
- a missing file or key is `None`;
- a file that is not valid TOML is an error, which `theos check` reports as "binaries not
  checked: <file>: <message>". It never fails the check.

The project layer never holds the binaries folder (data model §13). P14 builds the three layers
around this function.

### 5.11 JSON views

The views live in `theos_studio::views`, with `serde::Serialize`. P48 returns the same views over
MCP. Field names are snake case. Absent values are `null`, never left out, so every key is always
there.

```json
{
  "findings": [
    { "rule": "lore/never-confirm", "severity": "error", "path": "lore/clues/C-931.md",
      "id": "C-931", "line": 6, "column": 9, "span": { "start": 120, "end": 125 },
      "message": "…" }
  ],
  "errors": 1,
  "warnings": 0,
  "binaries": { "checked": false, "reason": "no binaries folder set" }
}
```

- **`check`:** the document above.
  - `binaries` is `{ "checked": true, "folder": path }` when a folder was used.
  - `--staged` adds `"changed": [paths]` and leaves out `binaries`.
  - `span` holds byte offsets into the record's LF text (P02).
  - A finding without a position has `line`, `column` and `span` `null`. A finding on an input
    file (`.theos/settings.toml`, `canon/formalism/_source.json`) has `id` `null`.
- **`ledger`:** `[{ "id", "path", "title", "target", "entries": [{ "state", "floor", "place",
  "gate", "clue", "clue_path", "placed_by": [{ "script", "beat" }] }] }]`. These are the fields of
  `TruthLedger` (P04).
- **`schema`:**
  - without a kind: `{ "kinds": [{ "name", "part", "id_rule", "placement" }], "rules": [{ "id",
    "severity", "class", "configured" }] }`;
  - with a kind: `{ "name", "part", "id_rule", "placement", "fields": [{ "key", "type",
    "required" }], "body": { "beats": [{ "key", "targets" }]|null, "notes": [targets]|null,
    "verbatim" } }`;
  - `placement` is `{ "folder": "lore/truths" }` or `{ "under": "parent" }`.
- **`new`, `hook install` and `formalism`:** as in §5.6, §5.8 and §5.9.

These shapes are a public contract. A renamed or removed key is a breaking change
(`CONTRIBUTING.md` §2); an added key is not.

### 5.12 Error reports and P04's follow-up

`theos formalism` printed a source error twice: `FormalismError::Source` writes the error into its
message and also returns it as `source()`, and `anyhow`'s `{:#}` prints the chain. It is a class,
not one variant. Nine variants in `theos-content` do the same:
- `FormalismError::Source`, `FormalismError::Io`;
- `IndexError::Sqlite`, `IndexError::Cache`, `IndexError::CacheInUse`;
- `ProjectError::Io`;
- `ReadError::Io`;
- `WalkError`;
- `FactError::Sqlite`.

Directive 3 asks to fix the class. `theos_studio::report(&dyn std::error::Error) -> String` walks
`source()`:
- it starts with the error's own text;
- it adds each cause with `": "`, unless the text so far already contains it.

Every error the CLI prints goes through it, and P48's error results too. The library errors stay
as they are: their own text is complete, which the editor's UI needs. A unit test feeds each of
the nine variants and checks that each cause appears exactly once.

### 5.13 Fixtures and tests

- **Projects:** the tests build small projects in temporary folders, with synthetic records marked
  `# synthetic fixture (P05), not canon`. Ids are in the 9xx range of P02–P04's synthetic
  records, and the texts are neutral ("Synthetic statement A"). Each test builds its own
  project, so ids cannot collide with other crates' fixtures.
  - The CE-17.2 fixtures: a `never` truth with a clue that reveals it, which is a never-confirm
    violation; a clean pair; and the T-012/C-031 shape with synthetic ids.
  - `theos new` takes the data model §14 example `mother-ilse` under synthetic parents.
- **Git in tests** runs the real `git` (CI runners have it). Each test repository is isolated:
  - `GIT_CONFIG_GLOBAL` points at an empty file and `GIT_CONFIG_NOSYSTEM=1`;
  - `user.name` and `user.email` are set with `-c`;
  - `core.autocrlf` is set per case.
- **Binary tests** run `env!("CARGO_BIN_EXE_theos")`. They install the hook with `theos hook
  install` and commit with `git commit`, so the real hook script runs through git's `sh`.
- **Per OS:** the mode 0755 is Unix-only code. Its test carries `#[cfg(unix)]`, the Windows test
  (no mode) carries `#[cfg(windows)]`, and the PR gets the labels `windows` and `macos`.

### 5.14 How each AC is proven

| AC | Test |
|---|---|
| CE-17.1 | **Per command, a binary test** on a fixture project: the text output, and `--json` parsed with `serde_json` and compared field by field:<br>- `check`: findings, counts, binaries checked and not checked, exit 0, 1 and 2;<br>- `ledger`: the P04 ledger as JSON;<br>- `new`: path from both placements, numbering, template and skeleton, a refusal leaves no file;<br>- `schema`: all kinds, one kind, rules with configured severities;<br>- `hook install`: the script, the marker, re-install, foreign hook, `--force` with backup, a repo-local `core.hooksPath`, a refused global one, a project in a subfolder, a linked worktree. |
| CE-17.2 | **Through git, with the installed hook:**<br>- a staged never-confirm violation is refused, and the message names `lore/never-confirm` and the file;<br>- the same violation with a fix only in the working tree is refused;<br>- a clean snapshot with a violation only in the working tree commits;<br>- `commit -a` and `commit <path>` check their own snapshots;<br>- the lowered target makes a committed clue overshoot and is refused;<br>- a staged settings change that tries to lower never-confirm is refused;<br>- a local record excluded through `.git/info/exclude` with a violation does not block;<br>- a project in a subfolder commits through its hook.<br>**Unit tests:** the path mapping and the change building of §5.7, with a project in a subfolder and files outside it. |

## 6. Interfaces other phases use

| Phase | Uses |
|---|---|
| P06 | `Workspace` (`open`, `find`, `index_mut`) with its own `impl` blocks; `introduced` (§5.9), built once by whichever of P05 and P06 builds first; moves `theos new` onto its create command |
| P46 | `Git::find()`, fallible from the start, gets the version check; `GitError` is `#[non_exhaustive]` and gets `TooOld` and a cancel variant; `run`'s signature stays; reads pass `--no-optional-locks` |
| P47 | `check_staged(&mut Workspace, &Git) -> Result<StagedCheck, StudioError>`, `blocks()`, `changed`; `staged::read` and `Staged` (§5.2). The commit holds the repo lock while git runs the hook; the check never takes it. The commit must not hold an index transaction meanwhile |
| P48 | `Workspace`, the views of §5.11, `report` for error results, and the contended-cache fallback |
| P49 | The finding line formats of §5.4 for the problem matcher, and the exit codes 0, 1 and 2. P49 adds `--version` to `theos` (and to `theos-mcp`) |
| P14 | `user_settings::binaries_root`, grown into the three layers of G14 |
| tower-of-theos | `templates/records/<kind>.template.md` with `{{title}}` is what `theos new` reads |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| A standing error in `HEAD` | Every commit is refused until it is fixed | That is CE-22.1 read literally, and it keeps errors from piling up. tower-of-theos has no findings today. The message lists the errors with their files. `git commit --no-verify` remains git's escape, and P49's check action is the backstop |
| The hook is slow on a cold cache | The first commit after a clone waits for a full index | Under a second for 500 records (CE-12.1). Later commits sync only changed files |
| `theos` moves after the hook is installed | The hook finds no program | Fallback to the PATH. A re-install updates the path |
| Older git without `--no-optional-locks` (2.15) | The reads fail | P46 sets the minimum version in `Git::find`. P05's tests run on the runners' git |
| Windows `sh` and paths with spaces or quotes | The hook fails to start | Single-quoted path with `'` escaped. A test path with a space and a `'` on Windows |
| A cache contended by another build | `IndexError::Contended` | The in-memory fallback (§5.3) |
| CRLF in staged blobs (`core.autocrlf`) | Positions or findings differ from the working copy's | `theos-content` reads CRLF as LF (P02). A test commits with `autocrlf=true` |

## 8. Review Focus — inputs most likely to break it

- **Changes against the working tree:**
  - a staged file edited again in the working tree;
  - a staged deletion of a file still on disk;
  - an untracked record;
  - a record that only `.git/info/exclude` or `core.excludesFile` keeps out of git;
  - a file deleted on disk but still staged.
- A truth staged in full with a lowered target (the case of §4.2).
- **Other indexes:** `commit -a`, `commit <path>`, and `commit` before the first commit.
- **Repository layout:** a project in a subfolder of the repository, with records outside it;
  two projects in one repository; a repo-local, a global and a tracked `core.hooksPath`; a linked
  worktree.
- **Settings:** `.theos/settings.toml` staged differently from the working copy, both a severity
  change and a guardrail lowering.
- **Entries that are not regular files:** symbolic links and submodule entries at record paths.
- **Paths:** with a space, with non-ASCII letters, or that are not UTF-8; an id that exists in
  another case; a record tracked with `git add -f` although the project's `.gitignore` keeps it
  out.
- **The hook script:** an existing foreign hook, a re-install, `--force`, an install path with a
  space or a `'`.
- **`theos new`:** a template whose body contains `{{title}}`; a compile template's
  `{{speaker}}`; a `--set` value the restricted YAML rejects; a parent that does not exist; the
  first id of a prefix; ids of different widths.
- **Output:** JSON with no findings, findings without a position, and findings on input files.
- **Exit codes:** 1 only for findings in `check`, 2 for failures; `ledger` and `formalism` moved
  from 1 to 2.
- **Error reports:** each cause exactly once, for each of the nine variants of §5.12.
- **`theos new` verdicts:** a standing error on a neighbour (passes), a second error of the same
  rule on a neighbour (refused), a message that changes with no new error (passes).

## 9. Rulings (Claude's calls)

- Ruling: the staged check covers the whole snapshot, computed by a new `Index::snapshot` instead
  of `preview`'s findings merged with stored ones — P06's lowered-target case needs the whole
  snapshot, and a staged settings change must apply to untouched paths too, which a merge with
  stored findings would miss — cost if wrong: one more function on the index.
- Ruling: `--staged` checks no binaries — binary findings are warnings and never block, and the
  hook should stay offline and fast — cost if wrong: a flag in the hook script.
- Ruling: findings on the commit's own paths print first — the author of the commit reads those
  first (P06's suggestion) — cost if wrong: an order.
- Ruling: exit code 1 means "check found errors" and 2 means "could not run"; P04's `ledger` and
  `formalism` move from 1 to 2 — CI and the hook must tell broken canon from a broken tool, and
  clap already exits 2 on usage errors — cost if wrong: two commands' failure code.
- Ruling: without a folder, commands find the project at or above the current folder — git and
  cargo do the same — cost if wrong: a command run in a nested project's subfolder finds the
  inner project.
- Ruling: the hook names its project folder relative to the repository's top folder, and one hook
  checks one project — git runs hooks in the top folder, a relative path fits every linked
  worktree, and a second project in the same repository is rare — cost if wrong: a hook that runs
  several checks.
- Ruling: `hook install` refuses a hooks folder outside the repository's git folder — a global
  `core.hooksPath` would refuse every commit outside a theos project, and a tracked one would
  commit a machine's path — cost if wrong: Jann installs the hook by hand in such a setup.
- Ruling: the hook calls the installing `theos` by absolute path, falling back to the PATH — GUI
  git clients often run hooks without the user's PATH — cost if wrong: a re-install after moving
  `theos`.
- Ruling: `hook install` never overwrites a foreign hook without `--force`, and `--force` keeps
  a backup — a hook may be someone's work, and deleting it is destructive — cost if wrong: one
  more file in the hooks folder.
- Ruling: `theos new` starts from the project's template, else from the kind — Jann keeps the
  templates for Obsidian, and a project without them still works — cost if wrong: one source to
  drop.
- Ruling: only `{{title}}` is replaced in a template — it is the only Obsidian placeholder the
  templates use, and a compile template's other `{{…}}` are content — cost if wrong: one more
  placeholder.
- Ruling: a new numbered id is the highest number of its prefix in the index plus one — data
  model §3 says numbers are never reused, and P07 brings block reservations for agents; a deleted
  highest number may be reused until then — cost if wrong: one reused number, caught in review.
- Ruling: `theos new` rejects only on errors it introduces, counted per path and rule, and writes
  on standing errors and warnings — Jann's ruling for P06's commands, so the verdict stays the
  same when `theos new` moves onto P06's create command — cost if wrong: none beyond P06's.
- Ruling: `theos schema` prints our own description of the kinds, not a JSON Schema — CE-17.1
  asks for `--json`, and the front matter is the data model's restricted YAML, which JSON Schema
  would describe only in part — cost if wrong: a second output format later.
- Ruling: the JSON views live in `theos-studio` and are not `Serialize` on `theos-content` types —
  they are a public contract (`CONTRIBUTING.md` §2), shared with P48 — cost if wrong: some
  mapping code.
- Ruling: the double error is fixed by one report function, not by changing nine library errors —
  the class needs one fix, and the library errors' own text stays complete for the UI — cost if
  wrong: the libraries' chains stay redundant for other printers.
- Ruling: on a contended cache the CLI answers from an in-memory index and warns — it needs two
  builds writing at once, which only happens while developing; per-build cache files would leave
  stale caches behind — cost if wrong: about a second per command while it lasts.
- Ruling: `binaries_root` comes from the user settings file at `dirs::config_dir()`, and
  `--binaries` overrides it — architecture §13 puts it in the user layer, and CI has no user
  settings — cost if wrong: one more source when P14 adds the layers.
- Ruling: `THEOS_CONFIG_DIR` overrides the user settings folder — on Windows `dirs` ignores
  `%APPDATA%`, so tests could not isolate the user layer otherwise, and a portable install can
  use it too — cost if wrong: one environment variable P14 keeps.
- Ruling: the snapshot takes the index's entries from `git ls-files -s` and treats every cached
  path that is not tracked as removed; it asks git for no untracked files — git's exclude sources
  differ from the index's walk, so a locally excluded record would count as committed — cost if
  wrong: one listing of the project's index entries per check.
- Ruling: a record tracked with `git add -f` although the project's `.gitignore` keeps it out is
  not checked — the index never reads such a file (P03), so for theos it is not a record — cost
  if wrong: such a file slips through the hook and through `theos check`.
- Ruling: git commands of the staged read run in the repository's top folder — git runs hooks
  there, and a relative `GIT_INDEX_FILE` is meant from there; the probe resolved it from a
  subfolder too, but no git documentation promises that — cost if wrong: none.
- Ruling: staged bytes are read by object id from `ls-files -s`, not by `:path` — no path, and so
  no newline in a path, ever reaches `cat-file`'s line-based input, and the mode tells regular
  files from links and submodules — cost if wrong: one more git call.
- Ruling: paths that are not UTF-8 are dropped from the snapshot — records and input files have
  ASCII names (data model §2, §3) — cost if wrong: such a file is not checked.
- Ruling: `theos formalism` gets `--json` although CE-17.1 does not list it — every command of the
  binary then behaves the same — cost if wrong: a few lines.

## 10. Left to the builder

- Module and type names inside the crates, as long as §5.2, §5.3, §5.7 and §6 hold.
- The parameter types of `Git::run`, as long as P46 and P47 can add to them without a change.
- Help texts, the exact notes and summary lines on stderr, and messages, within §5.
- Whether `which` or a PATH search of its own resolves `git`. `which` is preferred (directive 2).
- The test file layout under `crates/theos/tests/`, `crates/theos-studio/tests/` and
  `crates/theos-vcs/tests/`.
