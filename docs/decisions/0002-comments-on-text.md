# 0002 · Comments on text sections

A proposal for Jann. It decides how comments on passages of a record are stored, anchored,
threaded and offered over MCP, and which phases build them. It changes no acceptance criterion
(AC): the criteria in §9 and the wording in §10 are proposals for Jann to add to the umbrella
spec.

## Status

proposed · 2026-11-05 · accepted when Jann merges the PR
`docs(docs): comments on text sections (docs)`. On acceptance, Jann adds the criteria of §9 and
the wording of §10 to `docs/specs/00-umbrella.md` and `docs/architecture.md` in a PR of their
own.

Sources: the P10 comments probe ([notes][probe-notes], [behaviours][probe-tests]),
[decision 0001][0001], the P02 phase spec (`docs/specs/phases/P02-records-and-grammars.md`, in
its spec PR), and the probes of §3.5.

## Goal

Jann's words (2026-11-04 and 2026-11-05): commenting that feels like Claude's artifact
documents.

- Select a passage in a record and add a comment.
- Comments show in the margin next to their passage, as threads with replies.
- A thread can be resolved and reopened; each comment shows its author and time.
- Claude, through MCP, can easily list and read the open threads with the passage they point
  at, reply, and propose an edit that resolves a thread.
- Comments survive ordinary edits of the text around them. A comment whose passage is gone is
  shown as orphaned, not lost.

**Non-goals:** comments on images, maps or generator outputs (notes, data model §9.3, already
carry feedback for generators); several people typing in one record at once; notifications;
showing comments in Obsidian or VS Code.

**Anti-goals** (a failure even if everything else works): a comment action that changes a
record's bytes; a comment that silently attaches to the wrong passage; an agent that changes a
record through a comment.

## Recommendation in short

| Question | Recommendation | § |
|---|---|---|
| Storage | Beside the content, never inside it: one TOML file per commented record, `.theos/comments/<record id>.toml`, holding its threads and comments and edited in place. Git keeps the history. | 1, 2 |
| Anchor | A text quote with 32 characters of prefix and suffix, plus byte offsets and a hash of the record text: W3C Web Annotation's TextQuoteSelector with a TextPositionSelector. The editor maps anchors through every edit; after outside edits a reader finds the quote again. An anchor that is found neither exactly nor nearly is orphaned, never guessed. | 3 |
| Threads | `open` or `resolved`. Whether the passage is attached, changed or orphaned is computed on every read. Only people resolve and reopen. | 4 |
| MCP | Read: `comment_list`, `comment_read`. Agent writes: `comment_open`, `comment_reply`, `comment_propose`. | 5 |
| Agent replies | A reply is a comment, not a proposal. Every change to a record stays a proposal. This needs one sentence in G7 and one in CE-23.1 (§10). | 5.3 |
| Phases | P02 exposes positions; P05 builds the model, the files, re-anchoring and MCP read; P06 the commands; P07 the agent tools and the review link; P12 the margin; P13 Review and Changes. | 8, 9 |

## 1. Storage

**S1 Sidecar files in the working tree** (recommended). Comments live in tracked files under
`.theos/comments/` of the content project. The record never changes.

**S2 Markers inside the Markdown.** For example CriticMarkup `{==passage==}{>>comment<<}`, or
HTML comments `<!-- c:t-7f3a9c -->…<!-- /c -->` around the passage, with the thread either in the
marker or in a sidecar.

**S3 Sidecar files on a git ref.** The same files as S1, committed to a ref of their own
(`refs/theos/comments`) with git plumbing (`hash-object`, `mktree`, `commit-tree`,
`update-ref`) and never checked out.

| | S1 files in the tree | S2 markers in the Markdown | S3 files on a git ref |
|---|---|---|---|
| Canonical text (G5) | untouched | every comment action edits the record; a marker inside a link, a beat attribute or a callout title breaks the P02 grammar | untouched |
| The record's diff (CE-21.2) | clean; comments change their own file under `.theos/comments/` | comment noise in the record's field and word diff | clean; comments never show in Changes |
| Merges | an ordinary text conflict only when two machines change the same record's comments before a pull (§2) | conflicts inside prose when two sides comment near each other | needs our own sync: fetch, merge the two comment refs, push |
| Other readers of the text | see nothing | P02's grammar and round trip, the word check (G8 `inworld-vocab`), the ledger, note text sent to generators, compile previews and LLM prompts (G9) must all strip markers | see nothing |
| Agents | write comments into the comment file; CE-23.1 needs a sentence (§10) | a comment is a record edit, so a proposal, and impossible on canon records, which are locked against agents (CE-26.2) | write to the ref and touch no working tree; CE-23.1 holds as written |
| Anchoring | re-anchoring after outside edits (§3) | exact and free: the markers move with the text | as S1 |
| Travels with a copy of the record file | no; the thread names the record id | yes | no |
| Visible in git, GitHub, VS Code | yes | yes | no: GitHub does not browse refs outside `refs/heads`; only `theos` and the editor show them |
| Sync and backup | ordinary commit and push | ordinary | a plain `git push` leaves them behind; P06's push must push the ref and a fresh clone must fetch it |
| Cost | a reader and writer of one TOML file per record | a grammar, a round trip and stripping in five places | S1 plus git plumbing in `theos-vcs` |

**Recommendation: S1.** The text stays untouched (G5). S2's one advantage, free anchoring, is
matched closely enough by quote anchoring (§3), while S2 leaks markers into every reader of the
text and makes agent comments on canon impossible. S1 keeps "plain files, git history"
(architecture §1) and works with every git tool. S3 is the choice if Jann wants CE-23.1 to stay
as written: it keeps agents out of every working tree, at the price of comments that only the
editor and `theos` can see and sync.

**Where:** `.theos/comments/` in the content project, tracked.

- `.theos/` is already "not records" for the index (P01 spec §5), Obsidian hides dot folders,
  and tower-of-theos's `.gitattributes` already forces LF for `toml` (CE-01.2).
- Keyed by record id, not path. Moving a record to another folder changes nothing; renaming its
  id renames its comment file in the same command (CE-20.3, P06).
- Not beside the record (`lore/truths/T-012.comments.toml`): that puts non-record files into the
  kind folders, which P01's layout and P03's index would have to learn to skip, and moving a
  record to a new parent would have to carry them along.

## 2. Files

**One TOML file per commented record, edited in place:** `.theos/comments/<record id>.toml`. It
holds the record's threads and their comments. A record without comments has no file. Git keeps
the history.

```toml
# .theos/comments/T-012.toml — comments on T-012; written by the editor and `theos`
[[thread]]
id = "t-7f3a9c"              # "t-" and 6 random hex digits, unique per record
state = "open"               # open | resolved
opened_quote = "The binders call them ancestors"   # the passage when the thread was opened

[thread.anchor]
quote = "The binders call them ancestors"
prefix = "…the 32 characters before…"
suffix = "…the 32 characters after…"
start = 412                  # byte offsets into the record text (LF line endings), end exclusive
end = 443
seen = "9d6239…"             # SHA-256 of the record text the offsets refer to (64 hex digits)

[[thread.comment]]
id = "c-a41c"                # "c-" and 4 random hex digits, unique per thread
author = "Jann Erhardt"
by = "human"
at = 2026-11-05T14:02:11Z
text = """
Is this passage still right after the split?
"""

[[thread.comment]]
id = "c-0b7e"
author = "Claude"
by = "agent"
session = "s-0012"
at = 2026-11-05T14:05:40Z
text = """
I proposed a shorter wording as review entry r-17.
"""

[[thread.proposal]]
entry = "r-17"
summary = "Shorter wording of the passage"
status = "pending"           # pending | approved | rejected
```

**What a change does to the file**

| Action | Change |
|---|---|
| open a thread | a new `[[thread]]` table with its anchor and first comment |
| reply | a new `[[thread.comment]]` |
| resolve, reopen | `state`, and `resolved_by` and `resolved_at` when resolved |
| the passage changed (§3.4) | the values of `[thread.anchor]` |
| edit a comment | its `text`, and `edited_at` |
| withdraw a comment | `withdrawn = true`; its text is removed |
| an agent proposes an edit | a new `[[thread.proposal]]` |
| approval or rejection of that proposal | its `status` and `note`; an approval also resolves the thread |
| delete a thread | its table goes; when the last one goes, the file goes |

**Format.** TOML, because comment text has line breaks. The pinned `toml` 1.1.8 writes them as
readable multi-line strings, and writes nested threads and comments as `[[thread]]` and
`[[thread.comment]]` tables, and reads both back exactly (§3.5). Restricted YAML forbids
multi-line scalars, and JSON writes `\n` escapes. Files are read and written with `serde` and
`toml`; there is no format of our own. A reader ignores fields it does not know, so a new field
is not a breaking change.

**Writing.**
- Every write reads the file, changes it, and writes it whole and atomically (a temporary file
  renamed over it), while holding the repo lock (architecture §8). So a reply that an agent
  writes while Jann resolves a thread in the editor loses neither change. The editor never
  writes its copy from memory: it re-reads the file under the lock.
- The output is deterministic: fixed field order, threads in the order they were opened,
  comments in time order. A change therefore shows only its own lines in a diff. In the probe, a
  new reply added its lines and a new anchor offset changed one line, and nothing else moved.

**Why edited in place, and why one file per record.**
- **Git already keeps the history** (Jann, review of this record). An earlier draft used one
  append-only file per event, so that two branches or machines writing the same thread could
  never conflict. Under this record's own design that case is rare:
  - every writer works in the one working tree, the editor directly and agents through the
    command layer (§5.3), and the lock serialises them;
  - a git conflict needs two machines, or a branch, that change the same record's comments
    before a pull.

  The append-only design paid for that rare case with a growing pile of files.
- **A conflict that does happen** is an ordinary text conflict in a readable file. It is listed
  with its record, like any pull conflict (CE-36.3).
- **One file per record, not per thread:**
  - one read when a record opens;
  - one rename when its id changes;
  - the Changes screen groups by record anyway (architecture §7).

  One file per thread would only make the two-machine conflict rarer.

**Line endings.** The reader turns CRLF inside text values into LF. The probe shows that `toml`
keeps a CR that a Windows tool wrote into the file.

## 3. Anchors

### 3.1 What the editor can do

From the P10 probe, on CodeMirror 6 (decision 0001):

- A comment is a `Decoration.mark` range in a `StateField`. The library maps it through every
  change.
- `invertedEffects` makes adding, removing, and deleting all of the commented text undoable.
- Positions are file offsets (UTF-16), because the document is the file.
- All seven behaviours passed, in 71 lines:
  - adding and removing leave the text byte-identical;
  - an insertion before the range shifts it, an insertion inside grows it, a deletion over an
    end shrinks it, and typing at either end stays outside;
  - deleting all the commented text drops the comment, and undo brings it back;
  - an outside change maps it;
  - a fresh state restores it;
  - a range across a link behaves like prose.
- Not probed: drawing, overlapping comments, many threads, a comment inside a fenced block.

So while the editor is open, anchors need no algorithm of ours: CodeMirror maps them. Our part
is the text that changed while the editor was closed, or in another program.

### 3.2 Options

**A1 Offsets only** (the P10 probe). Exact while the editor is open. Wrong after any outside
edit before the passage: a git merge, an approved proposal, VS Code, Obsidian.

**A2 Quote with context, plus offsets.** W3C Web Annotation's TextQuoteSelector (`exact`,
`prefix`, `suffix`) with a TextPositionSelector as a hint, as hypothes.is does. Robust to edits
around the passage. An edited passage needs a near match.

**A3 A2 plus the old text.** Store the git blob id of the record text instead of a hash. When the
text changed, read the old blob with `git cat-file` and map the offsets through a diff of old and
new text. Exact through any outside edit. But every anchor write must also store the blob
(`git hash-object -w`), git prunes unreferenced blobs after two weeks, and re-anchoring then
needs git, which `theos-content` does not use.

**Recommendation: A2**, with the hash in `seen` so that an unchanged record skips the search.
A3's exact mapping is what the editor already does while it is open: CE-34.2 maps a reloaded
file as a change. What remains is an edit made while the editor was closed, and for that, A2
either finds the passage or says "orphaned". A3 can be added later; the only format change is
that `seen` holds a blob id.

### 3.3 Re-anchoring

A pure function in `theos-content`, `resolve(anchor, text)`, returning
`Attached { start, end, changed }` or `Orphaned`. MCP, the CLI and the editor on open all use it.

1. **Unchanged.** `sha256(text) == seen` and `text[start..end] == quote`: attached at the
   offsets.
2. **Exact quote.** The quote occurs in the text. One occurrence: attached. Several: the one
   whose surrounding text agrees with `prefix` and `suffix` over the most characters, then the
   one nearest `start`.
3. **Near quote.** The quote occurs nowhere. Search near `start` for a passage whose start
   matches the quote's first characters and whose end matches its last characters, allowing
   small differences (bitap, as in diff-match-patch's `match_main`, which hypothes.is uses).
   Then require the text between them to be at least 75 % similar to the quote (the ratio of the
   `similar` crate). Found: attached with `changed = true`, and the margin shows the stored quote
   next to the current text.
4. Otherwise **orphaned**.

A wrong attachment is worse than an orphan. Step 3 therefore errs towards orphaning, and the
phase that builds it tests near misses: a repeated phrase, a sentence moved to another
paragraph, one word changed, a passage rewritten. Step 3 is required (CE-51.2). P05 builds it on
a maintained Rust port of diff-match-patch if one passes its probe. Otherwise it builds it on
`similar` alone: a character diff of the quote against the text around `start` gives the best
matching range and its ratio. Our code is only the glue between the crate and the rules above.

**Anchor status** (computed on every read):
- `attached`: found by step 1 or 2;
- `changed`: found only by step 3, which happens only after an edit made outside the editor.
  The next save in the editor re-anchors it exactly;
- `orphaned`.

Separately, a thread knows whether its passage was **edited since it was opened**: the current
text differs from its `opened_quote`. The margin then shows that original quote, and
`comment_read` returns it.

**Inside the app** (P12): CodeMirror maps the ranges through typing, undo and outside changes.
Deleting the whole passage drops the mark, and the thread shows as orphaned until undo brings
the text back. **On save**, the editor updates the stored anchor of each thread whose live range
differs from it: its text differs from the stored quote, or `resolve` on the saved text would
not return the live range. After a save, every thread of the record is therefore attached
exactly. Threads whose passage did not move or change are left alone, so typing elsewhere in the
record writes nothing.

**After outside edits and merges**, git merges the record and the comment file, and every reader
runs `resolve`. A merge conflict in the record is resolved first; the anchors resolve on the
result.

**Orphaned** means one of these:

- the record no longer exists;
- steps 1 to 3 find nothing;
- the passage is empty.

An orphaned thread keeps its quote and its comments. It is listed under "Detached" in the margin
and in `comment_list`, and it can be re-attached to a new selection (a new anchor), resolved or
deleted. It is never dropped.

**Positions.** Offsets are bytes of the record's text, front matter included, with LF line
endings: the coordinates of every span in P02 (P02 spec §5.2). The editor converts them to and
from UTF-16 with P02's `TextIndex`. Prefix and suffix are 32 Unicode scalar values each
(hypothes.is uses 32). Version 1 offers selections in the body only; the model also allows the
front matter.

### 3.4 The life of an anchor, step by step

When a passage changes, the editor updates the `[thread.anchor]` values of its thread in the
record's comment file. Git keeps the earlier values.

1. **Opening a record.** The editor reads that record's comment file,
   `.theos/comments/<id>.toml`, and no other. It runs `resolve` (§3.3) for each thread on the
   text as loaded, converts the offsets to UTF-16 with P02's `TextIndex`, and puts one mark per
   thread into CodeMirror. Orphaned threads go to the Detached list.
2. **Typing.** CodeMirror maps every mark through every change (§3.1). No comment file is read
   or written:
   - typing before a passage shifts its mark;
   - typing inside it grows the mark;
   - deleting all of it drops the mark, and the thread shows as detached until undo.
3. **Saving.** The editor writes the record first. Then it asks, for each thread: would the
   stored anchor, resolved on the saved text, find exactly the live mark?
   - **Yes**, for example after typing elsewhere, or when the passage moved but its text still
     identifies it: nothing is written. The stored offsets and hash are then stale, on purpose.
     They are only hints, and the next read finds the passage by its quote (§3.3, step 2), one
     search in one record.
   - **No**: the editor updates the thread's anchor with the live mark's text as the new quote,
     its new prefix, suffix and offsets, and the hash of the saved text. Two cases lead here:
     - the passage's own text changed;
     - a repeated phrase whose surroundings changed so much that the stored anchor would now
       pick another occurrence.

   A save thus touches the comment file only when a commented passage itself changed: usually
   not at all, otherwise a few lines.
4. **Changes from elsewhere**, such as an agent's reply or a thread opened with `comment_open`.
   The editor watches the open record's comment file and reloads its threads when it changes
   (CE-54.4). A changed anchor re-places its mark.
5. **Outside edits**, such as an approved proposal, a git merge, or a change in VS Code.
   - While the record is open, CE-34.2 reloads a clean record as a change, and CodeMirror maps
     the marks through it as in step 2.
   - While it is closed, nothing happens until someone reads it. Then `resolve` places each
     thread, and the next save in the editor updates the anchors whose passage changed.
6. **MCP and the CLI never update anchors.** They run `resolve` on the record as it is on disk.
   Two readers therefore agree on where a thread points: `resolve` is one function, run on the
   same text. Agents write comments and proposals only (§5.2).

Project-wide lists, such as `comment_list` across all records, read every comment file; P05 may
cache that in `.theos/cache` like the index.

### 3.5 Probes for this proposal

- `toml` 1.1.8 on toolchain 1.99.1:
  - a comment with multi-line text (quotes, a backslash, a tab, non-ASCII, an emoji) and a
    datetime serialises with `'''` and `"""` strings, and deserialises equal;
  - a file of two threads with nested comments serialises as `[[thread]]`, `[thread.anchor]` and
    `[[thread.comment]]` tables and deserialises equal. Adding a reply and changing one anchor
    offset, then writing the whole file again, changed only those lines;
  - a CRLF copy of a file keeps the CRs inside the text, hence the rule in §2.
- The CodeMirror facts of §3.1 come from the P10 probe.
- Not probed: a Rust port of diff-match-patch, and `similar`. P05 probes them before its plan
  (engineering directive 2); step 3 of §3.3 needs at least `similar`.

## 4. Threads

- **State:** `open` or `resolved`.
  - `open` → `resolved`: a person resolves it, or Jann approves a proposal that names the thread
    (the approval marks the proposal approved and resolves the thread).
  - `resolved` → `open`: a person reopens it.
  - Agents never resolve or reopen.
- **Anchor status**, computed and never stored: `attached`, `changed` (found only by a near
  match, §3.3), or `orphaned`. It is independent of the state: an orphaned thread can be open.
  "Edited since opened" is a separate flag (§3.3).
- **Comments:** the first comment and every reply. Each has an author, `by` (human or agent), the
  session for agents, a time, and Markdown text in which `[[id]]` links show as chips.
  - Editing replaces a comment's text, which then shows "edited".
  - Withdrawing removes the text and shows "withdrawn" in the comment's place.
  - Only the author edits or withdraws a comment, and only people do.
- **Proposals:** a proposal shows in the thread as "Claude proposed an edit (r-17)", with its
  summary, a link to Review and its status (pending, approved, rejected with the note).
- **History:** who resolved a thread and when is in the file (`resolved_by`, `resolved_at`).
  Everything earlier is in git (`git log -p` on the comment file).
- **Replying to a resolved thread** needs a reopen first. Agents get "thread is resolved".
- **Who:** people by git's `user.name` (G6: git through its command line); agents as "Claude"
  with the session id (architecture §8).
- **Locked records:** comments on canon records and on `never` truths are allowed; they change
  no record. A proposal on them fails as every proposal does (CE-26.2).

## 5. MCP

### 5.1 Read tools (P05)

**`comment_list`**
- Input: `record` (an id, optional); `state` (`open` by default, `resolved`, `all`); `anchor`
  (`any` by default, `attached`, `changed`, `orphaned`).
- Output: `threads`, each with:
  - `thread` (`T-012/t-7f3a9c`), `record`, `record_title`, `state`;
  - `anchor`: `status`, `quote`, `current` (the current text, when changed), `line`, `column`,
    `end_line`, `end_column`, `edited_since_open`;
  - `opened_by`, `opened_at`, `last_at`, `comments` (a count);
  - `first`: the author and the first 200 characters of the opening comment;
  - `flags`: `locked` and `never`, as in CE-18.3.

**`comment_read`**
- Input: `thread`.
- Output: the fields of `comment_list`, plus:
  - `passage`: its `text`, and the paragraph around it as `before` and `after`;
  - `original_quote`: the thread's `opened_quote`, when the passage was edited since;
  - `proposable`: whether a session can propose on the passage (§5.2);
  - `comments`: `id`, `author`, `by`, `session`, `at`, `text`, `edited_at`, `withdrawn`;
  - `resolved_by`, `resolved_at`;
  - `proposals`: `entry`, `summary`, `status` (pending, approved or rejected), `note`.

Lines and columns follow P02 (from 1, columns in Unicode scalar values), so Claude can quote
them back.

### 5.2 Agent tools (P07)

**`comment_open`**
- Input: `record`, `quote` (the passage as text), `near_line` (optional), `text`.
- Output: `thread`.
- Fails when the quote is not in the record, or occurs more than once and `near_line` does not
  pick one ("found 3 times: give near_line").
- Jann did not ask for it. It lets Claude point at a passage while reviewing a record. Drop it if
  it is not wanted.

**`comment_reply`**
- Input: `thread`, `text`. Output: `comment`.
- Fails on a resolved thread.

**`comment_propose`**
- Input: `thread`, `replacement` (the new text of the passage), `note`.
- Output: `entry` (the review entry) and `findings`.
- An ordinary proposal through the command layer (architecture §5): it replaces the passage as
  attached now, runs every check, and records the proposal in the thread.
- Fails for an orphaned thread, for a locked record (CE-26.2), and on blocking findings, which
  it returns while committing nothing (CE-26.3).
- It edits the record as the session sees it, like every proposal: the session worktree on
  `agent/<session>` (architecture §8). The passage is resolved there. A passage that exists only
  in Jann's uncommitted text is not in the session's copy, so the tool fails with "the passage is
  not in the session's copy of the record: commit it first". `comment_read` says so in advance
  (`proposable`). Proposing on uncommitted text would change P07's session design, which this
  record does not do.
- For an edit wider than the passage, `codex_propose` gains an optional `resolves` list of
  thread ids, with the same effect.

Every agent tool of a paused session fails with "paused" (CE-23.3). All names follow G7's
pattern.

### 5.3 Is an agent's reply a comment or a proposal?

**A comment.** Why:

- **Comments are not content.** No rule, ledger, generator, compile preview or LLM prompt reads
  them, so G7's purpose is untouched: agents never change content without Jann's review. Every
  change to a record remains a proposal (`comment_propose`, `codex_propose`).
- **A proposal per reply breaks the conversation.** Each reply would wait in Review until Jann
  approved it on its own (CE-37.3 forbids bulk approval), so Jann would read Claude's answer
  only after approving it.
- **Canon records are locked against agents** (CE-26.2). As proposals, replies on canon passages
  would always fail, and canon is where questions matter most.

**Cost.** G7 says "agents only propose", and CE-23.1 says sessions "never touch the human working
tree". Under S1, an agent's comment is a change to a comment file in that tree. §10 proposes one
sentence for each. Under S3, only the G7 sentence is needed.

Agents only add comments and replies, and their proposals are recorded in the thread. The tools
allow nothing else: agents never change or remove anything else in a comment file, and never
resolve, reopen or delete a thread.

**If Jann prefers no change to G7 and CE-23.1**, replies become proposals. They are written on
the session branch and shown at once in the margin as pending, since the session worktree is on
disk, and they reach `main` when Jann approves the session's entry. Merging them can then
conflict with Jann's own comments on the same record (§2).

## 6. G9 and `never` truths

The MCP read tools already return `never` truths, marked (CE-18.3), and the comment tools mark
threads on them the same way. The editor's own LLM client (P17) never puts comments into a
prompt, so G9 is unaffected.

## 7. Undo

Comment actions are commands (architecture §5): opening, replying, resolving, reopening,
editing, withdrawing and deleting are each undone by one undo, which restores the comment file's
earlier bytes, as every undo does (CE-20.2). In the editor, undoing a text edit that orphaned a
thread brings the passage back, and the thread with it (§3.1).

## 8. Phases

| Phase | Builds | Where |
|---|---|---|
| P02 (now) | Spans as byte offsets into the LF text of the whole record; line and column (from 1, columns in Unicode scalar values); a `TextIndex` that converts offsets to line and column and to UTF-16 offsets, and back. No comment code. | `theos-content` |
| P05 | The thread model; reading comment files (`theos check` names a file that does not parse and a comment file whose record is gone); `resolve` (§3.3); the read tools `comment_list` and `comment_read`. | `theos-content`, `theos-mcp` |
| P06 | Comment commands with undo; writing comment files under the repo lock; renaming an id renames its comment file (CE-20.3). | `theos-studio` |
| P07 | `comment_open`, `comment_reply`, `comment_propose`, and `resolves` on `codex_propose`; approval marks the proposal approved and resolves the thread; rejection marks it rejected, with the note. | `theos-studio`, `theos-mcp` |
| P12 | Margin threads, commenting on a selection, live mapping, anchor updates on save, the Detached list, agent comments appearing while the record is open; the UI references for them (below). | `editor-ui`, `theos-editor`, `docs/ui/` |
| P13 | Review shows the threads an entry resolves; Changes lists comment files apart from records; the UI references for them. | `editor-ui`, `docs/ui/` |

**UI references** (G13, G18). `docs/ui/screens/` has no comment states yet. P12's spec PR adds
them to the screens that show the manuscript editor: margin threads with replies, the composer,
the Detached list. P13's spec PR adds threads to `07-review` and comment files to `08-git`. Both
are built from `design/theos-ui/` as G18 requires. They are protected paths, so they wait for
Jann with their spec.

**Why the quote anchor and re-anchoring are in `theos-content`:** they are pure functions of
text, shared by MCP, the CLI and the editor.

**Why P05 builds them, not P03 or P02:** P03's index needs nothing from comments, and P05 is the
first phase that serves them. None of P02's ACs needs comments, and building them there would
put an unproven feature into the phase the whole core track waits for. What P02 must not get
wrong is the coordinate system, because every stored anchor depends on it; that is all P02
builds.

## 9. Proposed acceptance criteria

For Jann to add to `docs/specs/00-umbrella.md`. The numbers are suggestions.

**P02**, only if Jann wants the conversion proven as an AC (the P02 spec builds it either way
once this record is accepted):
- CE-09.5 Every position in a parse result converts between byte offset, line and column, and
  UTF-16 offset, and back.

**P05 · CE-51 Comment threads**
- CE-51.1 A record's threads are read from `.theos/comments/<id>.toml`; a comment file that does
  not parse is a finding with its path, and its record still opens.
- CE-51.2 After an edit made outside the editor, a passage that moved is found again, an edited
  passage attaches as changed, and a deleted passage is orphaned, never attached elsewhere.
- CE-51.3 `comment_list` and `comment_read` work over stdio; `comment_read` returns the passage
  with its lines and every comment with its author and time.
- CE-51.4 Threads on locked records are flagged, and threads on `never` truths are marked, as in
  CE-18.3.

**P06 · CE-52 Comment commands**
- CE-52.1 Opening, replying to, resolving, reopening and deleting a thread, and editing or
  withdrawing a comment, are commands with undo; none changes a record's bytes.
- CE-52.2 A comment written by an MCP process while the editor writes another to the same file
  is kept, and so is the editor's.
- CE-52.3 Renaming an id renames its comment file in the same command.
- CE-52.4 Deleting a record keeps its threads, which show as orphaned.

**P07 · CE-53 Comments for agents**
- CE-53.1 `comment_open` and `comment_reply` add comments under the session's name and change no
  record.
- CE-53.2 `comment_propose` creates a review entry linked to its thread; approving it resolves
  the thread, rejecting it keeps the thread open with the note.
- CE-53.3 Agents cannot resolve, reopen, edit, withdraw or delete; the comment tools of a paused
  session fail with "paused".

**P12 · CE-54 Comments in the manuscript editor**
- CE-54.1 Selecting a passage and adding a comment opens a thread in the margin next to it;
  replying, resolving and reopening work there, and every comment shows its author and time.
- CE-54.2 A thread's passage follows typing, undo and outside changes; saving re-anchors exactly
  the threads whose passage moved or changed, and no others.
- CE-54.3 A thread whose passage is gone is listed as detached, with its quote, and can be
  re-attached or resolved.
- CE-54.4 An agent's comment or proposal appears in the open record without a reload and is
  marked as the agent's.

**P13 · CE-55 Comments in Review and Changes**
- CE-55.1 A review entry shows the threads its proposal resolves.
- CE-55.2 Changes lists comment files apart from record changes.

## 10. Wording for Jann (with the criteria)

- **G7.** Now: "MCP: stdio only, `rmcp`, tool names `^[a-z][a-z0-9_]*$`, agents only propose."
  Proposed: "MCP: stdio only, `rmcp`, tool names `^[a-z][a-z0-9_]*$`, agents only propose
  changes to records and settings; they may add comments (decision 0002)."
- **CE-23.1.** Now: "Two parallel sessions never touch the human working tree or each other."
  Proposed: "Two parallel sessions never touch the human working tree or each other, except to
  write comments (decision 0002)." Not needed under S3.
- **Architecture** §2 (`.theos/comments/` in the tree) and §9 (the five tools).
- **Data model:** a short section saying that comments are not records and live in
  `.theos/comments/`.

## 11. Risks

| Risk | Mitigation |
|---|---|
| Near re-anchoring attaches to the wrong passage | Errs towards orphaned; near-miss tests (§3.3); the margin shows a changed passage next to its stored quote |
| The editor and an MCP process write the same comment file at once, and one change is lost | Every write re-reads the file under the repo lock (§2); CE-52.2 tests it |
| Two machines change the same record's comments before a pull | An ordinary text conflict in a readable file, listed with its record (CE-36.3); rare for one author |
| An agent floods a record with comments | Agents only open and reply; pausing a session stops them; Jann can delete threads |
| The margin with many threads and overlapping passages is unprobed (§3.1) | P12 probes it before its plan |
| Comments of deleted records pile up | `theos check` lists comment files without a record; deleting one is one command |

## 12. If rejected

Jann names what to change: the storage (S2 or S3, or one file per thread instead of per
record), the anchors (A1 or A3), or replies as proposals (§5.3). This record is rewritten in the
same PR. P02 is affected only if its positions change. Under S2, P02's body grammar would have to
parse and round-trip the markers, which needs a new P02 criterion.

[0001]: 0001-prose-editor.md
[probe-notes]: ../../research/P10-prose-editor/app/src/probes/comments/NOTES.md
[probe-tests]: ../../research/P10-prose-editor/app/src/probes/comments/behaviours.ts
