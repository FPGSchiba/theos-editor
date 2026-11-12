# P06 · Command layer and repo lock — phase spec

Status: waiting for Jann's approval · 12 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P06, as of umbrella version 6. Version 5 (#33) split the
old "Command layer and git" into P06, P46 Git read and P47 Git write; version 6 (#34) split P05.
Scope and acceptance criteria come from there; this spec decides how.

## 1. Goal

After P06, every write to the files of a content project goes through one command layer in
`theos-studio`, and every writer holds one repo lock:

- A command is planned against the files on disk and checked as a whole with P03's `preview` and
  P04's rules. Then it is either rejected with nothing written, or written in full.
- A command is rejected when it introduces an error. Errors that were already there come back
  with it but do not block (Jann).
- Undo puts back the exact bytes of every file one command touched. The entries live in memory
  for as long as the workspace is open (Jann).
- Renaming an id rewrites the record, its file name, its child folder, its versions and every
  inbound link the index knows, in one command.
- The repo lock keeps writers in different processes apart: the editor, `theos` and every
  `theos-mcp`. A holder that crashes never leaves it held.

Architecture §5 draws the command layer as one path for every caller:

```text
command ─► schema check ─► validators (record + neighbours + requires chain) ─► route
   human  → write working tree → reindex → event (undo entry)
   agent  → write in the session worktree → commit on agent/<session> → review entry → event
```

P06 builds that path with the human route and leaves a seam for the agent route.

Later phases build on it:
- P05's `theos new` moves onto the create command (§6.1);
- P07 adds the agent route and proposes through the same commands;
- P11 edits records in the Codex, with undo, redo and records changed outside the editor;
- P12 saves the manuscript editor's text;
- P19 adds comment files to a rename;
- P47 takes the same lock for git writes.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-20 Command layer**
- CE-20.1 A blocking finding rejects the command and leaves files untouched.
- CE-20.2 Undo restores the bytes.
- CE-20.3 Renaming an id updates every inbound link in one command.

**CE-22 Git write and repo lock** (CE-22.1 and CE-22.3 in P47)
- CE-22.2 The repo lock keeps two writers apart across processes.

## 3. Scope

**Goals**
- In `theos-studio`:
  - the command pipeline: plan, base check, preview, verdict, write, reindex, finish (§5.4), with
    a route seam for P07 (§5.10);
  - six commands: create, write, edit, patch, delete and rename (§5.3, §5.8);
  - the undo journal, with undo and redo (§5.7);
  - whole-file atomic writes with rollback (§5.6);
  - the repo lock (§5.9).
- P05's `Workspace`, extended with the journal and the lock (§5.2). If P06's build starts before
  P05 has merged, P06 creates `Workspace` in the agreed shape, and P05 merges onto it.
- In `theos-content`, one public function: the edits that rename ids in a record, spelling each
  front-matter scalar in its own quoting style (§5.8). It belongs there because P02's YAML
  encoder is private to that crate.
- A `tracing` log line for each command, with its `Update` and `revalidated` included (P04
  ruling L21).
- Every part uses std or a crate already in `Cargo.lock` (§4.2, §5.1): `std::fs::File::lock`,
  `std::fs::rename`, `tempfile`, `sha2` and `serde_json`. P06 adds no crate to the lockfile.

**Non-goals**
- Git: version check, status, history and diffs (P46); staging, commit, push and pull (P47). The
  staged check (P05).
- The agent route: session worktrees, `agent/<session>`, review entries, and records locked
  against agents (P07). P06 builds the seam only.
- UI: undo menus, reloading or merging a record changed outside the editor, and a file watcher
  (P11, P12).
- Undo across restarts (Jann, §4.1).
- Moving a record to another parent. Filling a template: `theos new` builds the text, and the
  create command writes it.
- The editor-part registry (§9, ruling 12).
- Comment files (P19) and pins (P22). They join a rename through its participants (§5.8).
- Dispatching commands by name for IPC or MCP (P11, P07).

**Anti-goals** (P06 fails if any of these holds, even with every test green)
- A rejected command that changed a byte on disk or a row of the cache.
- An undo that writes anything but the exact earlier bytes, or that overwrites a change made
  after the command.
- A rename that leaves a link to an old id that the index knew, or that renames only some of a
  record's versions.
- A lock that stays held after its process ends, or a reader (query, `check_staged`, sync) that
  waits for it.
- A file left half-written by a crash during the write: every file is replaced whole, or not at
  all.
- Our own file locking or atomic write where std does it (directive 2).

## 4. Decisions

### 4.1 Decided by Jann (12 Nov 2026)

- **Ruling (Jann): three focused phases.** P06 is the command layer and the repo lock (CE-20,
  CE-22.2), P46 Git read (CE-21) and P47 Git write (CE-22.1, CE-22.3), in umbrella version 5
  (docs PR #33). The lock goes with its first writer: commands write under it, and P47's git
  writes take the same lock.
- **Ruling (Jann): undo lives in memory, per workspace.** It holds the last 100 commands and is
  lost when the workspace closes or its process ends. Beyond that, git is the history: P13
  discards or restores from it. Nothing is written to disk for undo, so nothing duplicates git.
- **Ruling (Jann): only errors a command introduces block it.** An error that the affected paths
  already had comes back with the command but does not reject it, so errors can be fixed one at
  a time. The commit check stays strict: any error in the staged snapshot blocks (P05, P47).
- **Agreed with the P05 session** (Jann asked the two specs to settle it):
  - P05 builds `theos_vcs::Git`, `theos_vcs::staged`, `theos_studio::Workspace` and
    `check_staged`, and P06 builds on `Workspace` (§6.1);
  - the hook and P47's commit call the same `check_staged`, which never takes the repo lock;
  - P05's own split takes phase numbers P48 and up.

### 4.2 Probes (12 Nov 2026, toolchain 1.99.1, Windows 11, git 2.56.1)

A scratch crate outside the repo, with the versions pinned in `Cargo.lock`.

- **The lock, with std `File::try_lock`** (stable since Rust 1.89):
  - while one process holds the lock on `lock`, a second process's `try_lock` returns
    `WouldBlock`, and so does a second handle in the holding process itself;
  - when the holder exits, the lock is free at once; after `taskkill /F` of the holder, too;
  - on Windows, another process cannot even read the locked file (os error 33), so the holder's
    name goes in a separate file (§5.9).
- **Atomic replace:**
  - `tempfile` 3.27.0's `NamedTempFile::persist` fails with "Access is denied" whenever any
    other handle has the target open, even a plain `std::fs::File::open` reader;
  - `std::fs::rename` of a temporary file over the target succeeds while a std reader holds it
    open, and that reader keeps reading the old bytes (POSIX semantics);
  - it fails only while a program holds the target open without delete sharing, with "Access is
    denied". After such a failure the temporary file is removed and the target is unchanged.

  So a file is written as a temporary file from `tempfile` in the same folder, kept, and moved
  with `std::fs::rename` (§5.6).
- **A create that never replaces:** `NamedTempFile::persist_noclobber` writes a new file; over
  an existing target it fails with `AlreadyExists` (os error 183), also when the target is
  spelled in another case, and it leaves no temporary file behind.
- **The content project today:** `../tower-of-theos` has no records yet and no findings
  (`Index::in_memory` and `sync`), so nothing there is a standing error.
- P03's `preview`, `update` and `backlinks`, P02's `Record::with_edits`, `Record::patch` and the
  link `target_span`s, and `KindSpec::order` are all in place on `main`.

The git probes of this brainstorming belong to P46 and P47 and are kept in §11.

### 4.3 Approaches considered

| Question | Chosen | Considered |
|---|---|---|
| Split | P06 command layer + lock, P46 git read, P47 git write (Jann) | the lock in P47 (command writes would get it later, changing P06's code); two phases, with all of git in one (about P04's size); no split |
| Undo storage | in memory, per workspace (Jann) | in `.theos/cache/undo/` (copies of uncommitted edits beside the files); as blobs in git's object store (the command layer would need git, and gc prunes them) |
| Blocking | errors the command introduces (Jann) | any error on the paths the command touches or revalidates: a standing error, or a neighbour with one, would block every command, saves included |
| What "introduced" means | more errors of a rule on a path than findings of it before | the same path, rule and message (messages change with edits, §5.5); a stable key for every finding in `theos-content` (work in P03's and P04's rules for one P06 question) |
| Undo entries | the bytes of each file before and after | inverse commands, such as setting the old field value back: "restores the bytes" is then a property to prove for every command, not a given |
| Undo order | the newest entry, or the newest entry that touched one record, allowed only while its files still hold the bytes it left | one global stack only (a per-record undo in P12 would undo an unrelated record) |
| Lock | std `File::try_lock` on `.theos/cache/lock` | `fd-lock` 4.0.4, `fs4` 1.1.0 (both wrap the same OS calls std now has); a lock file whose existence is the lock (stays behind after a crash) |
| Atomic write | `tempfile` + `std::fs::rename` | `NamedTempFile::persist` (fails on Windows whenever the file is open, §4.2); writing in place (a crash leaves a torn file) |
| Crash in the middle of a command | rollback on errors; no recovery journal | a write-ahead journal in the cache, which records each command before it runs (a second history beside git) |
| Command shape | a trait, one module per command | one enum of every command (a new command touches the enum, the pipeline and the dispatcher) |
| Freshness of the index | each command updates its own paths first; a rename syncs the whole project | a full sync before every command (one walk per keystroke-sized save) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Module (indicative) | Crate or tool | Our code, and why |
|---|---|---|---|
| Workspace | `workspace` | P05 | P06 adds the journal and the lock state (§5.2) |
| Commands | `command::{create, write, edit, patch, delete}` | P02's `Record`, `with_edits`, `patch`; P03's `KindSpec::order` | Each builds a plan (§5.3) |
| Base | `command::base` | `sha2` | SHA-256 of the bytes a caller saw (§5.3) |
| Pipeline | `command::pipeline` | P03's `update`, `preview`, `findings_for` | The order of §5.4 |
| Verdict | `introduced` (shared with P05's `theos new`) | – | Counts per path and rule (§5.5); one function, built by whichever of P05 and P06 builds first |
| Writing | `command::write` | `tempfile` (`persist_noclobber`), `std::fs::rename` | Order, retries, rollback (§5.6) |
| Undo | `command::undo` | – | The journal (§5.7) |
| Rename | `command::rename` | P03's `backlinks`, `children`, `record` | Id map, path map, participants (§5.8) |
| Rename edits | `theos_content::rename` (new, public) | P02's front-matter model and body links, P02's YAML encoder (crate-private, hence this module) | The `id` field and every link to a renamed id, as span edits (§5.8) |
| Repo lock | `lock` | `std::fs::File::try_lock`, `serde_json` | Waiting, the owner file (§5.9) |
| Routes | `command::route` | – | The seam for P07 (§5.10) |

New dependencies of `theos-studio`: `theos-content`, `tempfile` (a workspace dependency, so far
only for tests), `sha2`, `serde`, `serde_json`, `thiserror` and `tracing`. All of them are in
`Cargo.lock` already.

### 5.2 The workspace

`Workspace` is P05's (§6.1): one per working tree. Today that is the project root; in P07 it is
also each agent session's worktree. It holds the `Project`, the `KindRegistry` and the `Index`,
and opens with `Workspace::open(root)`, which syncs the index.

P06 adds two things to it:
- the undo journal (§5.7);
- whether this workspace holds the repo lock right now (§5.9).

Commands run as `workspace.run(command)`, and undo and redo as `workspace.undo(scope)` and
`workspace.redo(scope)`.

`workspace.read(path)` returns a file's bytes and its `Base`. A caller keeps the base of what it
showed the user, and hands it back with the command that changes the file.

### 5.3 Commands

A command is a type that implements `Command`:
- `label()`: the line shown in the undo list and the log;
- `plan(&PlanContext)`: the files it changes, read from the workspace as it is now. A plan is a
  list of `FileChange { path, before, after }`:
  - `path` is project-relative with `/`;
  - `before` is the bytes the file has now (`None`: it does not exist);
  - `after` is the bytes it will have (`None`: it is deleted).

**A new command is one module and one `pub use`.**

| Command | Input | Plan |
|---|---|---|
| `Create` | path, text | The file must not exist on disk: `std::fs::symlink_metadata` decides on the disk's own terms, so a case-insensitive disk also refuses a name that differs only in case. `after` = the text with LF line ends. |
| `Write` | path, base, text | `after` = the text with LF line ends. A text equal to the file's normalised text gives an empty plan, so saving an unchanged CRLF record writes nothing (P02's `was_normalized`). |
| `Edit` | path, base, `Vec<TextEdit>` | `Record::with_edits`. Spans are offsets into the record's LF text (P02). |
| `Patch` | path, base, `Vec<PatchOp>` | `Record::patch` with the `Order` of the record's kind (`KindSpec::order`). A record whose kind is not registered takes the order of its fields as they stand. |
| `Delete` | path, base | `after` = `None`. |
| `Rename` | id, new id | §5.8. |

- **Base check.** A command that takes a base compares it with the SHA-256 of the file's bytes on
  disk. When they differ, the command fails with `Stale { path }` and changes nothing. P11 and P12
  then reload or merge (architecture §5: "An external change to an open record reloads it if
  clean, or offers a merge if dirty").
- **Paths.** A path is relative, stays inside the project root, has no `..` component, and is
  not under `.git/`. A `\` is read as `/`. Any other path fails with `Path` before anything is
  read. Files that are not records may be written (`.theos/settings.toml` from P14, comment
  files from P19). The index ignores them, so the verdict finds nothing on them, except on the
  two input files of the lore step, which P03 treats as changes.
- **Edited records are written with LF.** A command that edits a CRLF record writes it with LF
  (P02; CE-09.4; the only exception to G5 that Jann has made). Undo puts the CRLF bytes back.
- A plan whose `after` equals `before` for every file is empty. An empty plan writes nothing,
  adds no journal entry, and returns an outcome with nothing changed.

### 5.4 The pipeline

`run` does these steps in order. Every step after the lock releases it on the way out, also on
an error.

1. **Lock.** Take the repo lock (§5.9).
2. **Plan.** `command.plan`. A rename syncs the whole index first (§5.8). Any other command first
   calls `index.update` on the paths it will read, so their rows match the disk.
3. **Base check.** For every change, read the file again and compare it with `before`. A
   difference is `Stale`: another program wrote the file between the plan and now.
4. **Preview.** `index.preview` with the plan's changes, as `Change::Content` and
   `Change::Removed`. Then the findings the same paths have now, read with `findings_for` on the
   unchanged cache, because `preview` rolls back.
5. **Verdict.** The errors the command introduces (§5.5). If there are any, return
   `Rejected { blocking, findings }`: no file is written, and the cache is as it was.
6. **Write.** Write the plan (§5.6).
7. **Reindex.** `index.update` on every path of the plan. Log its `Update` with `tracing`, every
   list included, `revalidated` too (P04 L21).
8. **Finish.** Hand the applied plan to the route (§5.10). The human route adds a journal entry
   and clears the redo list.
9. **Outcome.** `Outcome { changed, update, findings, introduced, entry }`:
   - `changed`: the paths written;
   - `update`: what the reindex did;
   - `findings`: every finding on the affected paths after the command, warnings and standing
     errors included;
   - `introduced`: the findings of `findings` that are new (warnings only, since new errors
     reject the command);
   - `entry`: the journal entry, when the route keeps one.

### 5.5 The verdict (CE-20.1)

- **Affected paths:** the preview's `reparsed`, `relinked`, `revalidated` and `removed`, and
  every path the plan gives content for.
- **After:** `Preview.findings`, with configured severities (P04).
- **Before:** `findings_for` of each affected path, read after the preview. A file the plan
  moves (rename, §5.8) has its findings looked up under its old path and counted under its new
  one.
- **Counting, not messages.** Findings are compared as counts per path and rule, not by message.
  Many P03 and P04 messages change with ordinary edits:
  - three-clue's "No clue moves T-012…" becomes "Only one clue…" when a first clue is added;
  - `link/dangling`, `schema/version-id`, `record/file-name`, reveal-order and requires quote
    ids or places.

  Matching by message would treat such an error as new. A truth with a standing three-clue error
  (set to error in the settings) could then never get its first clue through a command, and a
  rename would trip on every standing error that quotes the renamed id. Positions and
  severities play no part either.
- **Blocking:** for a path and a rule, the number of errors (severity after configuration) after
  the command is larger than the number of findings of that rule on that path before. The errors
  above the count before are the blocking ones; when several are candidates, the ones whose
  messages have no equal before are reported first.
  - A new file has no findings before, so every error on it blocks.
  - Guardrails are always errors (G8), so a command that adds a never-confirm, no-overshoot or
    inworld-vocab error to a path is always rejected.
  - A settings change that lowers a guardrail gives P04's `settings/guardrail` error on the
    settings file, which had none, so the change is rejected.
  - A settings change that raises a rule from warning to error does not block: the findings
    were there before, now with a higher severity.
- **Not blocking:** standing errors, and every warning. They come back in `Outcome.findings`,
  and the new warnings also in `introduced`. One command may also swap one error for another of
  the same rule on the same path: the count stays, so it passes (ruling 7).

Neighbours count (P04 §1: "P06 rejects a command whose preview has blocking findings, the
neighbours' included"). Lowering a truth's `target` so that an existing clue overshoots
introduces a no-overshoot error on the clue, so the change is rejected.

### 5.6 Writing files

- **One file:**
  1. Create the parent folders.
  2. Write `after` to a temporary file from `tempfile` in the same folder (prefix `.theos-`,
     suffix `.tmp`), and flush it to disk.
  3. A file that exists (`before` is `Some`): keep the temporary file and `std::fs::rename` it
     over the target. The rename is atomic on every OS of G2. On Windows it also replaces a file
     that another program has open with delete sharing (§4.2).
  4. A new file (`before` is `None`): `persist_noclobber`, which fails with `AlreadyExists` if a
     file has appeared at the target since the check, under any case on a case-insensitive disk
     (§4.2). A create never replaces a file.
- **Permissions.** A replaced file keeps its permissions. A new file gets the permissions a
  plain `File::create` would give, not `tempfile`'s owner-only mode.
- **Retries.** A rename refused with `PermissionDenied`, or with Windows' sharing violation, is
  retried 5 times, 100 ms apart. A program that holds the file without delete sharing is
  usually an indexer, a virus scanner or another editor saving. After the last attempt, the
  command fails with `Write { path, source }`, and the message says that another program has the
  file open.
- **Order.** First every file that gets content, in plan order, then every removal. Folders that
  removals of this command emptied are removed last. Other empty folders are left alone. A
  failure part-way never loses the only copy of a moved file.
- **Rollback.** On a failure, the files already written are put back from the plan's `before`
  in reverse order: restored bytes, or the file removed if it was new. Then the command fails
  with `Write`.
  - When the rollback itself fails, the error is `Partial { path, source, unrestored }` and
    names every file that differs from before.
  - Either way, step 7 still reindexes the plan's paths, so the cache matches the disk.
- **A crash during a command** can leave some files of a multi-file command written and others
  not. Each file is whole, because each is replaced in one rename. There is no recovery journal
  (§4.3). Git shows the half-done state, and `theos check` finds the links it breaks (§7).

### 5.7 Undo and redo (CE-20.2)

- **Entry.** One per applied command:
  - an id and the command's label;
  - every `FileChange` with its exact `before` and `after` bytes. A moved file is two changes:
    a removal and a creation;
  - the finding counts per path and rule of the affected paths, before and after the command
    (§5.5).
- **Journal.** Two lists per workspace: done and undone.
  - A new command appends to done and clears undone.
  - Done keeps the newest 100 entries and drops older ones.
  - The journal is never written to disk, and it ends with the workspace.
- **Scope.** `Scope::Newest`, or `Scope::Record(path)`: the newest entry that touched that path.
  P12 undoes per record. A command that touched several records (a rename) is undone as a whole
  from any of them.
- **Precondition.** Every file of the entry must still hold the entry's `after` bytes: present
  with those bytes, or absent where `after` is `None`. Otherwise the undo fails with
  `Changed { path }`, writes nothing, and the entry stays.
  - So an undo never overwrites a later change.
  - Undoing out of order works only where no later command touched the same files.
- **Undo** runs the pipeline of §5.4 with the reversed plan (each `after` back to `before`). The
  plan comes from the entry, so step 3 compares against the entry's `after`.
- **The verdict of an undo** counts the state the undo restores as standing. For each path and
  rule, the allowed number of errors is the larger of:
  - the count now;
  - the entry's count before the command.

  So undoing a fix brings its error back, because that error was there before the command. An
  undo that would add an error that neither state had is rejected like any command. One
  example: removing a record that a later command linked to.
- On success, the entry moves from done to undone.
- **Redo** is the same in the other direction:
  - its precondition is that every file holds the entry's `before`;
  - its verdict allows the larger of the count now and the entry's count after the command;
  - on success, the entry moves back to done.
- **The exact bytes.** Undo writes `before` as it was read, CRLF, missing final newline and all.
  It never writes the normalised text.

### 5.8 Rename (CE-20.3)

`Rename { id, to }` renames a record's id. That is its file name and its `id` field (data model
§2, §5), and every place that follows from it.

1. **Fresh index.** Sync the whole project (`index.sync`). A rename reads every record that
   links to the id, so it must see links added outside the editor.
2. **Checks.**
   - `id` names exactly one record: none is `NotFound`, several is `Ambiguous { paths }`.
   - `to` differs from `id`, also when case is ignored. Every built-in id rule fixes the case
     of its letters, so a rename that changes only case can never give a valid id. It fails at
     once with `CaseOnly` on every OS, before it could trip over the record's own file on a
     case-insensitive disk.
   - Once the maps of steps 3 and 4 are built: no record has a new id of the id map, and no new
     path of the path map exists on disk, the child folder `<folder>/<to>/` included. Existence
     is checked as for `Create`, hidden and ignored files included. Otherwise the error is
     `IdTaken { id, path }` for the first one taken.
   - Whether `to` fits the kind's id rule is left to the verdict: the preview gives P03's
     `schema/id` error, which is new, so the rename is rejected with P03's message.
3. **Id map.** `{id → to}`, extended by the participants:
   - **versions:** for a `concept` or `map`, each child record of kind `concept-version` or
     `map-version` whose id is `<id>-v<number>` gets `<to>-v<number>`. Otherwise every version
     would break `schema/version-id`, an error.
4. **Path map.**
   - The record moves to `<folder>/<to>.md`.
   - Its child folder `<folder>/<id>/`, if there is one, moves to `<folder>/<to>/` with every
     file in it, records or not, hidden and ignored files too (`std::fs::read_dir`, not the
     index's walk). World and people children and versions sit there (data model §2;
     `Placement::UnderParent`).
   - A version renamed by the id map moves inside the moved folder to its new file name.
5. **Edits.** The index's `backlinks` of each old id name the source records, and the renamed
   records are sources too. Each source is read and parsed with P02 at its old path, edited, and
   written at its new path. The edits come from one new public function of `theos-content`,
   `rename::edits(&Record, &IdMap) -> Result<Vec<TextEdit>, …>`, applied with
   `Record::with_edits`. It returns:
   - an edit of the `id` field's scalar when the record's id is in the map;
   - an edit for every link whose target is in the map:
     - in the front matter, the scalar alone, in a field, a list item (block or flow), a map
       entry or a lens entry. The new id is spelled in the scalar's own quoting style with P02's
       encoder, so `"T-012"` stays double-quoted, `'T-012'` single-quoted and `T-012` plain;
     - in the body, only the link's `target_span`, so `#fragment` and `|shown text` stay;
     - this covers every slot: fields, list items, lens entries, beat attributes, voice
       speakers and beliefs, note version and `resolved by` links, and mentions;
     - a record's links to itself are included.

   The function never goes through the patcher. The patcher re-encodes a changed list item
   without its quoting and rewrites the item's lines, dropping a trailing comment. Span edits
   leave every other byte as it is (G5).
6. **Participants.** The steps above are participants: `versions`, `children`, `ids` and
   `links`. Each is one module with one job, and the participants are one list. P19 adds
   `comments` (`.theos/comments/<id>.toml`, decision 0002), and P22 adds whatever pins need
   beyond what the index knows as links. **A new participant is one module and one line.**
7. **One command.** All of it is one plan, one preview, one verdict, one write and one journal
   entry. Binaries are not moved: their references are paths in the binaries folder (G10), and
   those stay valid.

Afterwards, no record holds a link to an old id that the index knew. Text that is not a record
(a README with `[[id]]`, notes outside the project) is not rewritten (ruling 3).

### 5.9 The repo lock (CE-22.2)

- **File.** `.theos/cache/lock` under the project root (architecture §8). `.theos/cache` is
  ignored by git (CE-01.3). An agent worktree's workspace locks the project root's file, not its
  own (P07).
- **Taking it.** `RepoLock::acquire(project_root, purpose, wait)`:
  1. create the folder and the file if they are missing;
  2. call `File::try_lock`;
  3. on `WouldBlock`, try again with a growing pause (10 ms doubling up to 200 ms) until `wait`
     has passed;
  4. then fail with `Busy { owner, waited }`.

  Commands wait up to 10 s.
- **Owner.** After taking the lock, the holder writes `.theos/cache/lock.owner` as JSON: its
  process id, program name, purpose (a command's label) and start time. A waiter reads it for
  the message, for example: "the project is busy: theos-mcp (pid 4242) has been running `rename
  T-012` for 3 s". The owner file is informational:
  - the holder deletes it before releasing;
  - a holder that crashed leaves it behind, and the next holder overwrites it;
  - `Busy` shows the owner only when it is readable.
- **Release.** Dropping the guard deletes the owner file and unlocks. The OS also releases the
  lock when the process ends or is killed (§4.2), so a crash never leaves it held.
- **Who takes it.**
  - Every command, undo and redo of the human route (P06).
  - Every git write (P47).
  - Agent commits and merges (P07).
  - Readers never: queries, `index.sync` and `update` outside a command, `check_staged` and the
    hook (§4.1). The index guards its own SQLite file with its busy timeout (P03).
- **Re-entry.** A workspace that already holds the lock and asks for it again gets `Reentrant`
  at once, instead of waiting for itself. A second handle in the same process would get
  `WouldBlock` (§4.2).
- **What it does not stop.** Programs that do not know it: Obsidian, VS Code, git in a
  terminal. The base check (§5.3) and the check right before the write (§5.4 step 3) catch their
  writes to the files a command touches. Git's own `index.lock` guards git against them (P47).

### 5.10 Routes: the seam for P07

The pipeline is the same for every route. A route decides three things:
- **where** the files are: the workspace root, either the project root or a session worktree;
- **finish:** what happens after the write. The human route adds the journal entry. P07's agent
  route commits on `agent/<session>` and adds a review entry, and keeps no journal;
- **lock:** the human route holds the repo lock for the whole command. P07 decides when the
  agent route holds it: it writes its own worktree, and commits touch the shared `.git`.

`Route` is a trait with those methods. P06 builds `HumanRoute`, and P07 adds `AgentRoute` with
no change to the pipeline or the commands. Each command goes through the same preview and
verdict on both routes. That is "every write passes the same validation" (architecture §1).

### 5.11 Errors and logging

- `CommandError` (`thiserror`), each variant with what a user needs:
  - `Rejected { blocking, findings }`;
  - `Stale { path }`, `NotFound`, `Exists { path }`, `IdTaken { id, path }`,
    `Ambiguous { id, paths }`, `CaseOnly { id, to }`, `Path { path, reason }`;
  - `Changed { path }`, `NothingToUndo`, `NothingToRedo`;
  - `Record` (P02's errors), `Patch`, `Edit`;
  - `Write { path, source }`, `Partial { path, source, unrestored }`;
  - `Index` (P03's `IndexError`, `Contended` included), `Lock` (`LockError`).
- `LockError`: `Busy { owner, waited }`, `Reentrant`, `Io { path, source }`.
- Logging with `tracing`:
  - `info` for each command applied or rejected, with its label and paths;
  - `debug` for its `Update`, every list included (L21);
  - `warn` for retries and rollbacks.

### 5.12 Fixtures

`crates/theos-studio/tests/fixtures/`: small synthetic projects (marked synthetic), built from
the data model's examples (§14) with no new lore:
- **`commands/`:** a truth, two clues that reveal it, a script with beats that link the truth,
  an npc with a lens, a floor with a child location, and a concept with three versions and a
  note. It holds every link slot of §5.8.
- **`standing/`:** a record with a standing error and a neighbour with one, for §5.5.
- **`crlf/`:** a CRLF record and a record without a final newline, for §5.7.

Tests copy a fixture into a temporary folder and run commands on the copy.

### 5.13 How each AC is proven

Tests carry `// AC: <id>` directly above a `#[test]` function (P00's `ac-trace`).

| AC | Proven by |
|---|---|
| CE-20.1 | (a) A patch that sets an invalid value, an edit that adds a dangling link, and a clue that newly reveals a `never` truth are each rejected with their blocking findings. After each, every file of the project has its bytes (hashed before and after), `dump()` of the index is unchanged, and no journal entry exists. (b) Lowering a truth's `target` so that an existing clue overshoots is rejected with the clue's finding: a neighbour. (c) A command on a record with a standing error, and a command whose neighbour has one, are applied, and `findings` lists the standing errors. With three-clue set to error, a truth with no clue gets its first clue although the message of its standing error changes. A create whose new record has an error is rejected. (d) A settings write that lowers a guardrail is rejected; one that raises a rule from warning to error is applied. |
| CE-20.2 | (a) For each command kind, undo gives every touched file its exact earlier bytes and the index equals a cold build. The CRLF record and the record without a final newline come back byte for byte. (b) Undo of a rename restores every path, removes the new folder and brings back the old one. (c) An undo whose file changed afterwards fails with `Changed` and writes nothing. (d) Redo gives the later bytes again. (e) `Scope::Record` undoes the newest entry of that record and leaves a newer entry of another record alone. (f) Undo of a command that fixed a dangling link brings the link and its error back; undo of a create that a later command linked to is rejected. (g) A property test: a random sequence of commands, then undo until the journal is empty, gives every file its first bytes. |
| CE-20.3 | Renaming the truth, the concept and the floor of `commands/` in one command each:<br>• no record holds a link to an old id (`backlinks` of each old id is empty, and every expected link now names the new id);<br>• every slot of §5.8 is rewritten, with fragments, shown texts and quoting kept (double-quoted, single-quoted and plain items in block and flow lists), and a trailing comment on a list item stays;<br>• the floor's child folder and the concept's versions moved and were renamed, and the versions' own inbound links (`picked`, `from`) were rewritten;<br>• the index has no `link/dangling` and no new finding;<br>• exactly one journal entry was added.<br>A rename to a taken id fails with `IdTaken`, a case-only rename with `CaseOnly`, and a rename to an id that breaks the kind's rule is rejected with `schema/id`. Unit tests of `rename::edits` in `theos-content` cover each slot and quoting style on its own. |
| CE-22.2 | Two processes (the test binary started again in a helper mode, or a small helper binary):<br>(a) the helper holds the lock, and the test's `acquire` with a short wait fails with `Busy` naming the helper's pid and purpose; after the helper exits, and in a second run after it is killed, `acquire` succeeds at once;<br>(b) the helper and the test each append a line to the same record 20 times with `Edit`, reading the base before each and trying again on `Stale`; at the end the record holds all 40 lines, each exactly once, and every intermediate write was whole. |

Untagged tests cover the rest:
- path checks;
- the write order and the rollback, with a target held open without delete sharing (Windows) or
  in a folder without write permission (Unix);
- permissions kept;
- `Reentrant`;
- the owner file;
- an empty plan.

The lock, the rename over open files and the permissions differ per OS. So the build PR gets the
labels `windows` and `macos` and links their runs, and `cargo xtask gate` runs on Windows before
the phase is claimed done (CLAUDE.md). The gate as merged applies. Its floor for `theos-studio`
is in `gate/thresholds.toml`, and no gate PR is needed.

## 6. Interfaces

### 6.1 From P05 (agreed on 12 Nov 2026 by both spec sessions; P05 spec PR #35, §4.2 and §6)

| Item | What P06 relies on |
|---|---|
| `theos_studio::Workspace` | One per working tree. It holds `Project`, `KindRegistry` and `Index`, opens with `Workspace::open(root)`, takes the registry from `parts::builtin()`, and gives `&mut Index`. P06 adds fields and `impl Workspace` blocks in its own modules. |
| `check_staged` | Never takes the repo lock. P47's commit, which holds the lock, runs git, and git runs the hook. |
| `theos new` | P05 writes the file after a `preview`, blocking only on introduced errors, and creates it with no-clobber (`create_new`), as `Create` does. Whichever of P05 and P06 merges second moves it onto `Create`; its behaviour stays the same. |
| `theos_studio::introduced(before, after)` | The counting of §5.5, one function for `theos new` and the command verdict, built by whichever of P05 and P06 builds first. |

If P06's build reaches `Workspace` before P05 has merged, it builds against these signatures, and
whichever PR merges second merges `origin/main`.

### 6.2 For other phases

| Phase | Uses |
|---|---|
| P05 | `Create` for `theos new` (§6.1) |
| P07 | The pipeline with an `AgentRoute`: finish = commit on `agent/<session>` + review entry, no journal. The commands as the steps of a proposal. `Rejected` for CE-26.3. `RepoLock` on the project root for agent commits and approvals. One `Workspace` per session worktree. |
| P11 | `run`, `undo`, `redo` with `Scope`; `read` and `Base`; `Stale` for records changed outside the editor; `Outcome.findings` and `introduced` for the findings panel |
| P12 | `Write` or `Edit` for saves; `Scope::Record` for undo in the editor |
| P14 | `Write` of `.theos/settings.toml`: the verdict rejects lowering a guardrail |
| P19 | Rename participants for comment files; commands on files that are not records |
| P22 | Rename participants, if pins name ids the index does not see as links |
| P47 | `RepoLock` for staging, commit and pull. After a pull, the journal's precondition (§5.7) keeps undo from overwriting what the pull wrote. |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| A crash or power loss part-way through a multi-file command | Some files written and others not, e.g. a rename with half its links rewritten | Each file is whole (one rename per file). Writes come before removals, so no file's only copy is lost. Git shows the state, and `theos check` finds the broken links. No recovery journal (§4.3). |
| Programs that ignore the lock (Obsidian, VS Code, git in a terminal) | A write between the plan and the write | Base check at the plan and again right before the write (§5.4). The window that remains is the write itself. |
| Windows: a file held open without delete sharing | Write fails | Retries, then rollback with a message naming the file (§5.6) |
| The index misses an outside edit before a command | The verdict compares against stale neighbours | Each command updates its own paths first, and a rename syncs everything. The commit check and the hook are strict (P05, P47). |
| Counting per path and rule lets one command swap an error for another of the same rule on the same path | That one new error passes the command verdict | The commit check and the hook block on every error of the staged snapshot (P05, P47), so it cannot be committed. Counting was chosen because many messages change with ordinary edits (§5.5). |
| A rename of a parent with many children | A large plan and preview | Correct by construction. Previews are incremental (P03, P04). The test project measures one rename of 50 files, and the plan records the time. |
| Undo memory | 100 entries with the bytes of every file they touched | Records are small text files. A rename of 200 files at 5 KB is 2 MB. |
| P05 and P06 build `Workspace` at the same time | A conflict on merge | One agreed shape (§6.1). The second to merge merges `origin/main`. |

## 8. Review Focus — inputs most likely to break it

- A CRLF record edited, then undone: CRLF must come back. A record without a final newline, the
  same.
- `[[T-012#3|the truth]]`, `[[T-012|T-012]]`, `[[T-012]]` twice in one line, and a link inside a
  voice block, a beat attribute, a note title (`resolved by [[…]]`) and a lens `unaware_of` list.
- Front-matter links in a block list, a flow list `[T-012, T-013]`, a quoted scalar `"T-012"`
  and a single-quoted one, and a block-list item with a trailing comment.
- With three-clue set to error: the first, second and third clue of a truth, each as its own
  command. The standing error's message changes each time.
- A rename where the record links to itself; a rename of `mother-ilse` whose children are npcs
  with grandchildren; a concept whose versions link each other through `from` and are `picked`.
- A rename to an id that only a hidden file, or a file ignored by git, already has.
- A rename that changes only case (`T-012` to `t-012`), on a case-insensitive disk and on Linux.
- A create whose target appears, written by another program, between the check and the write.
- A rename whose child folder holds a file that is not a record.
- A command on a record changed on disk after the caller read it, and changed again between the
  plan and the write.
- An undo after a later command touched one of its files, and after a git pull rewrote one.
- An undo of a command that fixed an error; a redo of it afterwards.
- A redo after a new command.
- An undo of a create that a later command linked to.
- Two processes, the holder killed with `taskkill /F` or `SIGKILL` while it holds the lock.
- A command whose observer, or a nested call, asks the same workspace for the lock again.
- A target file open in another program without delete sharing; a read-only target file.
- A record with a standing error edited elsewhere; a neighbour with a standing error
  revalidated; a settings change that raises a warning to an error.
- A plan that changes nothing: an unchanged save, a patch that sets a field to its value.
- Paths: `../outside.md`, `C:\x.md`, `/abs.md`, `.git/config`, `lore\truths\T-012.md`.

## 9. Rulings (Claude's calls)

1. Ruling: a rename cascades to the child folder and to the versions — data model §2 puts
   children and versions in a folder named after the parent, and `schema/version-id` makes a
   version's id start with its parent's — cost if wrong: a rename of a parent with versions
   would always be rejected.
2. Ruling: a rename moves every file under the parent's folder, records or not — a file left
   behind would sit in a folder named after an id that no longer exists — cost if wrong: one
   filter in the `children` participant.
3. Ruling: "every inbound link" means every link the index knows. Text that is not a record (a
   README, notes outside the project) is not rewritten — the index is the one place that knows
   links, and P03 decides what is a record — cost if wrong: one participant for the files to
   add.
4. Ruling: commands carry the base hash of the bytes the caller saw, and a mismatch is `Stale` —
   architecture §5 asks to reload or offer a merge on an outside change, and that is the UI's
   (P11, P12) — cost if wrong: none for P06.
5. Ruling: files are written whole through a temporary file: `std::fs::rename` for a file that
   exists, `persist_noclobber` for a new one. Writes come before removals, rollback restores
   the plan's `before`, and there is no recovery journal — §4.2, §4.3, git is the history — cost
   if wrong: a journal in the cache later.
6. Ruling: the lock is std's `File::try_lock`, polled, with the owner in a separate file;
   commands wait up to 10 s; re-entry fails at once; readers never lock — §4.2 — cost if wrong:
   one constant.
7. Ruling: an error is introduced when a command raises the number of errors of a rule on a
   path above the number of findings of that rule there before. Messages, positions and
   severities play no part, and a moved file's old findings count under its new path. The
   function is shared with P05's `theos new` — messages change with ordinary edits
   (three-clue's count, quoted ids), positions move with every edit, and a raised severity is not
   a new finding — cost if wrong: a swapped error of the same rule on the same path passes the
   command and is caught at commit (§7).
8. Ruling: each command updates the index on its own paths before planning, and a rename syncs
   the whole project — a rename must see every link, while a save must stay cheap — cost if
   wrong: a neighbour edited outside the editor is compared stale until the next sync.
9. Ruling: `Create` and `Write` turn line ends into LF, and an unchanged save writes nothing —
   G5, CE-09.4, P02's `was_normalized` — cost if wrong: none.
10. Ruling: undo and redo go through the full pipeline, and `Scope::Record` picks the newest
    entry that touched the record. Their verdict counts the state they restore as standing (the
    entry's counts before or after the command), so undoing a fix brings its error back —
    an undo is a write like any other, "undo restores the bytes" must also hold for a fix, and
    P12 undoes per record — cost if wrong: none.
11. Ruling: commands are a trait, one module each. A serialisable request type for IPC and MCP
    comes with P11 and P07 — no P06 caller dispatches by name — cost if wrong: one enum later.
12. Ruling: P06 does not build the editor-part registry that P03 §4.1 placed "from P06".
    `Workspace::open` keeps P05's `parts::builtin()`. P06's commands are core commands, not a
    part's, and nothing in P06 registers per part. The registry comes with the first phase
    that registers something per part beyond kinds and rules: P09's generator commands or
    P11's rail presets — cost if wrong: a small registry module in the next phase that needs it.
13. Ruling: no event bus. `Outcome` is the event; P11 forwards it to the UI, and other processes
    see changes through the files (P11's watcher) — cost if wrong: an observer list later.
14. Ruling: `Outcome.findings` lists every finding on the affected paths after the command, and
    `introduced` the new ones among them — architecture §5: "warnings come back with it" — cost
    if wrong: none.
15. Ruling: the rename edits come from a new public function in `theos-content`
    (`rename::edits`), as span edits that spell each scalar in its own style — P02's YAML
    encoder is private to that crate, and the patcher re-encodes changed list items without
    their quoting and drops a trailing comment — cost if wrong: none; P06's build PR changes
    `theos-content` code, not a protected path.
16. Ruling: a rename that changes only the case of an id fails at once with `CaseOnly` — every
    built-in id rule fixes its case, and a case-insensitive disk would otherwise report the
    record's own file as taken — cost if wrong: a two-step move for case-only renames later.

## 10. Left to the builder

- Module names and the split of `Write` and `Edit` code.
- The helper for the two-process tests: the test binary started again with an argument, or a
  helper binary in the crate.
- The exact backoff, the retry count and the wording of the messages, within §5.6 and §5.9.
- How the owner file stores the time.
- Whether `Patch` of a record with an unregistered kind keeps the order of its fields, or fails.

## 11. Carried forward to P46 and P47

These came out of this brainstorming, before Jann split the phase. They are not P06's design. They
are written down here so P46's and P47's specs start from them.

**Agreed with P05** (P05's spec, "Interfaces other phases use"):
- `Git::find()` is the one entry point, fallible from day one. P46 adds the CE-21.3 version
  check inside it.
- `Git::run` takes stdin bytes and extra environment variables, and passes the rest through.
- `GitError` is `#[non_exhaustive]`, with stderr kept whole. P46 and P47 add variants.
- Every read passes `--no-optional-locks`.
- `staged::read` and `check_staged` cover the whole staged snapshot; any error blocks.
  `check_staged` reads the snapshot's findings through P05's new `Index::snapshot(changes)`, and
  treats every cached path that git does not track as removed (`Index::paths()`).
- For P47: the commit must not hold an index transaction while git runs the hook, because the
  hook syncs the same cache.

**Probes** (12 Nov 2026, Windows 11, git 2.56.1):
- **Word diff:**
  - `similar`'s `from_words` and `from_unicode_words` count whitespace as tokens. A re-wrapped
    line shows as changes, and shared spaces stitch unrelated edits together.
  - `capture_diff_slices` over whitespace-free word tokens gives exactly
    `git diff --word-diff --word-diff-regex='[^[:space:]]+'`: a re-wrap disappears, and
    "under an open sky." → the new sentence remains.
  - `similar` is already a workspace dependency and works on texts that are not in git
    (previews, proposals).
- **History:** `git log --follow` keeps the two earlier commits across a rename that also
  rewrites `id:` (R091).
- **Status by record:** an unstaged rename shows as a deletion plus an untracked file. With a
  temporary copy of the index (`GIT_INDEX_FILE`) and `git add -N`, `status --porcelain=v2 -z`
  pairs them as `R91`, and the real index stays untouched.
- **Process trees:**
  - killing `git` (via an alias that runs `sh`, then `sleep`) with `Child::kill` left `git.exe`,
    `sh.exe` and `sleep.exe` running;
  - `process-wrap` 10.0.1 (watchexec) with `JobObject` on Windows killed all three; on Unix it
    uses `ProcessGroup`;
  - its `KillOnDrop` wrapper exists only for tokio.
- **Minimum git:**
  - `--no-optional-locks` needs 2.15, `status --porcelain=v2` 2.11 and `commit --trailer` 2.32;
  - P07's check of a merged result without touching the working tree would use
    `merge-tree --write-tree`, from 2.38;
  - macOS 13's command-line tools ship 2.39, and Ubuntu 22.04 ships 2.34;
  - suggestion: 2.38.
