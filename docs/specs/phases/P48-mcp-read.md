# P48 · MCP read — phase spec

Status: waiting for Jann's approval · 27 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P48, version 8. Scope and acceptance criteria come from
there; this spec decides how.

## 1. Goal

After P48, `theos-mcp` is an MCP server over stdio that Claude Code starts in a content project.
It answers six read tools from the project's records:

- `project_info`: what the project is: kinds, rules, counts, guides;
- `codex_search`: full-text search over every record;
- `codex_read`: one record with its text, links, backlinks, findings and, for a holder, its
  effective lens and beliefs;
- `codex_list`: the records of a kind, a status or a folder;
- `codex_graph`: the records around one record, by its links;
- `ledger_check`: the reveal ledger of the truths, with the findings of the reveal rules.

It also serves the content project's guides (`tools/guides/*.md`) as MCP prompts
(architecture §9).

Every result marks locked records and truths with target `never` (CE-18.3), so Claude can see
what it may read but never change, and what it must never confirm.

The logic lives in `theos-studio`, next to P05's views, so the editor (P11) and later CLI commands
can use the same queries. `theos-mcp` only maps tools to studio calls.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-18 MCP read tools**
- CE-18.1 `project_info`, `codex_search`, `codex_read`, `codex_list`, `codex_graph`,
  `ledger_check` work over stdio from Claude Code.
- CE-18.2 Claude answers "what does Mother Ilse believe" from records alone.
- CE-18.3 Locked records are readable and flagged; truths with target `never` are marked in every
  result.

## 3. Scope

In scope:
- the six read tools of CE-18.1 and the guides as prompts;
- the marks of CE-18.3 (`never`, `locked`) on every record a result names;
- one new query in `theos-content` (§5.4) and the studio queries and views (§5.5–§5.10);
- the `theos-mcp` binary: project resolution, the stdio server, error results, logging (§5.2,
  §5.3, §5.11);
- the tests of §5.13 and one recorded Claude Code session as evidence for CE-18.2.

Not in scope:
- the other read tools of architecture §9 (`review_list`, `gen_catalogue`, `gen_runs`,
  `atlas_read`, `quest_flow`, `brief_compose`, `pipeline_list`): their phases add them (P07, P09,
  P22, P24, P26, P27), each as one module and one registration line (§5.2);
- every propose tool and the session registry (P07);
- `--version` for `theos-mcp` (P49);
- writing the guides themselves (P28).

## 4. Decisions

### 4.1 Decided by Jann (27 Nov 2026)

| Question | Choice |
|---|---|
| How a long-running server stays current | Each tool call first runs `Index::sync`, P03's walk that reindexes only changed files. No file watcher |
| `never` truths over MCP | Returned in full and marked. G9 covers the prompts the editor builds (P17's drafts, generator briefs), as P03 §5.11 ruled; Claude needs the text to avoid confirming a `never` truth |
| Proof of CE-18.2 | An automated stdio test on the `mother-ilse` fixture, plus one recorded Claude Code session attached to the PR |
| Guides | `project_info` lists them and the server serves each as an MCP prompt now |

### 4.2 Probes (27 Nov 2026, toolchain 1.99.1, Windows 11)

- **rmcp 3.12.0** (current; P05's research probed 3.9.0) with the features `server`, `transport-io` and
  `macros`, and `tokio` with `rt`, `io-std` and `macros`, builds on the pinned toolchain.
  - `#[tool_router(server_handler)]` on an `impl` block generates the `ServerHandler`, so the
    `#[tool_handler]` attribute and the `#[expect(clippy::unused_async_trait_impl)]` that 3.9.0
    needed are not needed. When the server also serves prompts, the builder writes the `ServerHandler` impl and
    keeps the tool router (§5.2).
  - A tool that returns `Json<T>` with `T: Serialize + JsonSchema` lists an `outputSchema` in
    `tools/list` and answers with `structuredContent` plus the same JSON as a text block. Option
    fields appear as `"type": ["…", "null"]` and stay `required`, which matches P05's "every key
    is always present".
  - The server exits when stdin closes.
- **`theos-mcp` is still an empty `fn main() {}`** (P05 built the CLI only), so P48 builds the
  binary from scratch.
- **`cargo deny check licenses bans`** with this repository's `deny.toml` passes for rmcp, tokio
  and schemars 1 and their 74 crates. `deny.toml` needs no change.

### 4.3 Approaches considered

| Part | Chosen | Rejected and why |
|---|---|---|
| MCP library | `rmcp` (G7) | Hand-written JSON-RPC: G7 names `rmcp`, and directive 2 |
| Where the queries live | `theos-studio` (`codex` module) with views in `views` | In `theos-mcp`: the editor's Codex (P11) needs the same queries, and P05 put the views in the studio for this reason |
| Result format | `Json<View>` with an output schema; the same JSON as text | Text only: clients that read `structuredContent` would lose the shape; Markdown: a second format to keep in step |
| Freshness | `Index::sync` per call (Jann) | `notify` watcher: missed events and per-OS behaviour; open once: stale after every edit |
| Marks | One `Marks` table built per call from the index (§5.5) | Read each record's file to judge it: a list of 500 records would read 500 files; a field on `RecordSummary`: ties P03's type to MCP policy |
| Locked policy | In `theos-studio` (`marks`) | In `theos-content`: "locked against agents" is a rule of the command layer and of P07's agent route, not of the records |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Tool or crate | Custom code because |
|---|---|---|
| MCP protocol, stdio, tool schemas, prompts | `rmcp` 3.12 with `tokio` (current-thread runtime) | — |
| Input and output schemas | `schemars` 1, the major version `rmcp` uses (1.2.3 in the probe's lock), as a direct dependency of `theos-studio` and `theos-mcp` | — |
| JSON | `serde`, `serde_json` | — |
| Arguments of the binary | `clap` (derive) | — |
| Logging | `tracing`, `tracing-subscriber` to stderr | — |
| Project, kinds, index, search, links, lenses, ledger | `theos-content` (P02–P04), `Workspace` (P05) | — |
| Field values for the marks | one new query `Index::field_texts` (§5.4) | the index stores them; no public query reads them yet |
| Marks, graph walk, tool results | `theos-studio` | they are this phase's own rules |
| Guide titles | `pulldown-cmark` (first heading) | — |

New workspace dependencies: `rmcp = "3.12"` (features above), `tokio = "1"` (features above) and
`schemars = "1"`. The studio derives `JsonSchema` without depending on `rmcp`. If rmcp moved to
another `schemars` major, the views would no longer satisfy its `Json<T>` bound and the build
would fail, so a mismatch cannot pass unnoticed.

### 5.2 `theos-mcp`: the server

```text
theos-mcp [--project <folder>]
```

- **Project:** resolved once at start, with the same rules as `theos` (P05 §5.4): `--project`
  names the root, which must hold `theos.toml` and is never searched upward; without it, the
  nearest folder at or above the current one that holds `theos.toml` (`Workspace::find`).
  Claude Code starts a project's MCP server in the project folder, so `.mcp.json` needs no
  argument.
- **A failed start does not end the process.** When no project is found or the workspace cannot
  open, the server still answers `initialize` and `tools/list`, and every tool call returns an
  error result with the `report` of that error (§5.11). Claude Code then shows a reason instead
  of "failed to connect". The workspace is opened again on the next call, so `theos new` or a
  checkout that creates `theos.toml` needs no restart.
- **Shape:** one `Server` struct holds `Arc<Mutex<State>>`, where `State` is the workspace or
  the error that kept it from opening. The tool router lives in `tools/mod.rs`; each tool is one
  module under `tools/` with its parameter struct and one function that calls the studio. A later
  phase adds a tool as one module and one line in the router. The `ServerHandler` impl adds the
  server info, the instructions text (one paragraph naming the marks of §5.6) and the prompts of
  §5.10.
- **One call at a time.** The runtime is current-thread, and each call locks the state, runs
  `Index::sync` (§5.3), runs the query and returns. A call never holds the lock across an
  `.await`.
- **stdout carries only the protocol.** Logs go to stderr through `tracing-subscriber`, filtered
  by `THEOS_LOG` (default `warn`). A test checks that every stdout line is a JSON-RPC message.
- **Exit:** 0 when stdin closes; 2 when stdio itself fails.

### 5.3 Freshness and the shared cache

- Each tool call starts with `workspace.write(|index| index.sync())`. A sync on an unchanged
  project walks the files and compares sizes and times; P03's CE-12.1 bounds it.
- The cache may be held by another process (the editor, `theos check`, a second Claude). P05's
  `Workspace::open` then falls back to an index in memory, and `Workspace::write` switches to one
  when a later sync finds the cache contended. The server keeps that index for the rest of the
  session and syncs it like the cache; `project_info` reports `"cache": "memory"`.
- Only an error that the fallback cannot fix (building the index in memory fails, SQLite fails)
  becomes an error result for that call; the next call tries again.

### 5.4 `theos-content`: `Index::field_texts`

```rust
/// The valid scalar values of `field` (one row per record that sets it), ordered by path.
pub fn field_texts(&self, field: &str) -> Result<Vec<FieldText>, IndexError>;

pub struct FieldText { pub path: String, pub text: String }
```

It reads the `field_values` rows with `shape = 'scalar'`, `valid = 1` and no map entry. It is a
plain query of the cache; nothing new is stored.

### 5.5 `theos-studio`: the marks (CE-18.3)

`Marks::load(&Index) -> Result<Marks, StudioError>` reads, once per call:
- the paths whose `target` is `never` (`field_texts("target")`, kind `truth`);
- the paths whose `status` is `canon` (`field_texts("status")`);
- the paths whose `state` is `locked` (`field_texts("state")`, kind `style`);
- the records under `canon/formalism/` (the generated mirror, data model §3).

A record is **locked** when any of these holds; its `locked` list names each reason, in this
order: `"canon"`, `"never"`, `"formalism"`, `"style"`. These are architecture §9's "never over
MCP" records. A record is **`never`** when it is a truth with target `never`.

`Marks` answers by path and by id. By id, a duplicated id is `never` or locked when any record
with that id is, so a mark is never lost to a duplicate.

### 5.6 What every result carries

Every record a result names, as a hit, a list row, a graph node, a link's source or target, a
backlink, a belief, a ledger truth or clue, or the read record itself, carries:

```json
{ "never": false, "locked": [] }
```

- the two keys are flattened into the object that names the record, next to its `id`;
- a link to an id with no record has `never: false` and `locked: []`, plus `"exists": false`;
- the read record's text is returned in full, also for a `never` truth (§4.1).

A test walks the JSON of every tool's result and checks that each object with an `id` that names
a record also has both keys, so a tool added later cannot forget them (§5.13).

### 5.7 The views

New views in `theos_studio::views`, with `Serialize` and `schemars::JsonSchema`, under P05's rules
(snake case, every key present, `null` for absent). P05's views get `JsonSchema` too, so
`ledger_check` can return the ledger view. `JsonSchema` is an added derive, not a change of any
key. `RecordRef` is the shared part:

```json
{ "id": "mother-ilse", "path": "people/hearth-folk/fates-cult/mother-ilse.md", "kind": "npc",
  "title": "Mother Ilse", "status": null, "never": false, "locked": [] }
```

- **`project_info`:** `{ "root", "schema_version", "cache": "file"|"memory", "records",
  "kinds": [{ "name", "part", "records" }], "rules": [{ "id", "severity" }], "errors",
  "warnings", "guides": [{ "name", "title", "path" }], "tools": [names] }`.
- **`codex_search`:** `{ "query", "hits": [RecordRef + { "score", "matches": [{ "column",
  "line", "column_number", "match", "text" }] }], "truncated" }`. `column` is `"title"`,
  `"body"` or `"field:<key>"` (`"field:<key>.<entry>"` inside a map). `line` and
  `column_number` are P03's `position` (from 1, columns in Unicode scalar values); `match` is the
  record's text under the match's `span`; `text` is P03's `line`, without markers.
- **`codex_read`:** `{ "records": [Read] }`, one entry per record with the id (several for a
  duplicated id, which P02's rule already reports). `Read` is RecordRef + `{ "text", "links":
  [Link], "backlinks": [Link], "children": [RecordRef], "findings": [P05's finding view], "lens":
  Lens|null, "ledger": P05's ledger entry|null }`.
  - `text` is the record's file read with `read_record` (P02: LF, front matter included) right
    after the call's sync, so it matches what the index just read. A file deleted between the
    sync and the read is an error result.
  - `Link` is `{ "slot", "fragment", "line", "column", "source": RecordRef, "target": RecordRef
    + { "exists" } }`.
  - `lens` is set for `people`, `faction` and `npc`: `{ "chain": [RecordRef], "fields": [{
    "field", "value", "from": RecordRef, "overrides": [{ "value", "from" }] }], "beliefs": [{
    "belief": RecordRef, "through": RecordRef, "distorts": RecordRef|null, "accuracy" }] }`.
    `value` is a string or a list of ids. `distorts` and `accuracy` come from the belief's fields,
    so a belief that distorts a `never` truth names it with its mark.
  - `ledger` is set for a truth.
- **`codex_list`:** `{ "records": [RecordRef], "total", "offset", "truncated" }`.
- **`codex_graph`:** `{ "root", "depth", "nodes": [RecordRef + { "distance", "exists" }],
  "edges": [{ "from", "to", "slot", "fragment" }], "truncated" }`.
- **`ledger_check`:** `{ "truths": [P05's ledger entry + marks], "findings": [finding view],
  "errors", "warnings" }`.

These shapes are a public contract, like P05's (`CONTRIBUTING.md` §2): a renamed or removed key is
a breaking change; an added key is not.

### 5.8 The tools and their parameters

Parameters are JSON objects with `deny_unknown_fields`, so a misspelled parameter is an error, not
a silent default.

| Tool | Parameters | Result |
|---|---|---|
| `project_info` | none | §5.7 |
| `codex_search` | `query` (string, required), `limit` (1–100, default 20) | P03's `search`; `truncated` when it returned `limit` hits |
| `codex_read` | `id` or `path` (exactly one) | §5.7; an unknown id or path is an error result |
| `codex_list` | `kind`, `status`, `folder` (all optional), `offset` (default 0), `limit` (1–500, default 100) | the records that match all given filters, ordered by path |
| `codex_graph` | `id` (required), `depth` (1–3, default 1), `direction` (`"out"`, `"in"`, `"both"`; default `"both"`) | §5.9 |
| `ledger_check` | `id` (optional: one truth) | P04's ledger and the findings of the reveal rules |

- `folder` is a project-relative folder with `/`; a record is in it when its path starts with
  the folder and a `/`.
- `kind` must be a registered kind and `status` a value of P02's `status` enum; anything else is
  an error result that names the allowed values.
- The descriptions of the tools say what a result marks and that the server is read-only; they
  are the text Claude reads to pick a tool, so the plan writes them out.

### 5.9 `codex_graph`

- Breadth-first from `id`. Out-edges are `links_from(path)` of every record with that id; in-edges
  are `backlinks(id)` and `children(id)`. Each edge is `{ from, to, slot, fragment }` with ids.
- Distance counts edges from the root. Nodes at `depth` are listed, but their edges to nodes
  outside the result are not.
- At most 200 nodes. When the walk would add more, it stops at the end of the current distance,
  drops that distance's extra nodes in path order, and sets `truncated: true`.
- A link to a missing id is a node with `exists: false`, never walked further.

### 5.10 `ledger_check` and the guides

- **`ledger_check`:** `Index::ledger()` (or `ledger_of(id)`) as P05's view, each truth with its
  marks, plus every finding whose rule id starts with `lore/` on the paths of those truths and
  of their clues (today `never-confirm`, `no-overshoot`, `reveal-order`, `requires`,
  `three-clue` and `inworld-vocab`). P04's registry has no "reveal" class, so the prefix is the
  selector; a new lore rule appears without a change here.
- **Guides:** every `tools/guides/*.md` file of the project, not recursive, ordered by name.
  - `name` is the file stem; `title` the text of the first heading (`pulldown-cmark`), else the
    stem.
  - `prompts/list` lists them with the title as description. `prompts/get` returns the file's
    text as one user message. They take no arguments.
  - The folder is read on each `prompts/list` and `prompts/get` call, so new guides appear without
    a restart. A missing folder means no guides, not an error.
  - A guide name that does not exist is an MCP `invalid_params` error.

### 5.11 Errors

- **Error results:** a failed query (unknown id, a bad filter value, the index failing, no
  project) returns a tool result with `isError: true` and one text block: the `report` of the
  error (P05 §5.12). Claude reads it and can retry.
- **Protocol errors:** only for what `rmcp` itself refuses (unknown tool, parameters that do not
  match the schema, an unknown prompt).
- Studio errors are new `StudioError` variants (`UnknownRecord`, `InvalidFilter`) with
  `thiserror`; `theos-mcp` uses `anyhow` only in `main`.

### 5.12 Read-only

P48's server writes nothing in the project. The only file it may change is the cache, through
`Index::sync`, as `theos check` does. Every tool sets the MCP annotation `readOnlyHint: true`.

### 5.13 Fixtures and tests

- **Fixture project:** `crates/theos-mcp/tests/fixtures/codex/`, a small project with
  `theos.toml`, marked `# synthetic fixture (P48), not canon` where it is not a data-model
  example:
  - the data model §14 records `mother-ilse`, `C-031` and `S-004`, with synthetic parents
    (`fates-cult`, `hearth-folk`) and places, as P05's fixtures did;
  - synthetic beliefs with neutral texts ("Synthetic belief A"): `B-911` held by `fates-cult`, so
    Mother Ilse holds it through her parent, and `B-912` held by `mother-ilse`, which distorts the
    synthetic `never` truth `T-911`. The data model names `B-016` but defines no record for it,
    so the fixture does not invent one;
  - a canon record, a locked style, a formalism mirror record, a link to a missing id, a
    duplicated id;
  - two guides in `tools/guides/`.

  Tests copy it to a temporary folder, so the cache never lands in the repository.
- **Studio unit tests** for `Marks`, the graph walk (depth, direction, the 200-node cap,
  missing ids), the list filters, the views' keys, and `field_texts`.
- **Binary tests** start `env!("CARGO_BIN_EXE_theos-mcp")` with piped stdio and speak JSON-RPC
  through `rmcp`'s client (feature `client`, dev-dependency only), so the tests exercise the
  same stdio path as Claude Code.
- **The `CE-08.1` smoke test** passes `Stdio::null()` as stdin, so the server sees end of input
  and exits 0 instead of waiting on the test's terminal. Its assertion is unchanged.

### 5.14 How each AC is proven

| AC | Test |
|---|---|
| CE-18.1 | **Binary tests over stdio:** `initialize`, `tools/list` names exactly the six tools with input and output schemas and `readOnlyHint`; each tool called on the fixture and its `structuredContent` compared field by field; error results for an unknown id, a bad filter and a start outside a project; a record edited on disk between two calls shows the edit in the second (§5.3); stdout holds only JSON-RPC lines; `prompts/list` and `prompts/get` serve the guides. **Evidence:** the PR links a Claude Code session (`.mcp.json` with `theos-mcp`) that calls every tool on a copy of the fixture. |
| CE-18.2 | **Binary test:** `codex_search "Mother Ilse"` finds `mother-ilse`; `codex_read` on it returns the chain `mother-ilse → fates-cult`, the beliefs `B-911` (through `fates-cult`) and `B-912` (through `mother-ilse`, distorting `T-911`), each with its title and its `distorts`; `codex_read` on `B-911` returns its text. Nothing outside the fixture's records is read. **Evidence:** the recorded Claude Code session asks "what does Mother Ilse believe" with only `theos-mcp` as a tool and answers from those results; the transcript is attached to the PR. |
| CE-18.3 | **Binary tests:** `T-911` is `never: true` and `locked: ["never"]` in `codex_search`, `codex_read` (also as the `distorts` of a belief and as a link target), `codex_list`, `codex_graph` and `ledger_check`; the canon record, the style and the mirror record carry their reasons; `codex_read` of each locked record returns its full text. **The schema walk** of §5.6 runs over every tool's result. |

## 6. Interfaces other phases use

| Phase | Uses |
|---|---|
| P07 | The `Server`, its router and the one-module-per-tool layout for the propose tools; `Marks` and its `locked` reasons to refuse agent writes to locked records |
| P09, P22, P24, P26, P27 | The same layout for `gen_*`, `atlas_read`, `quest_flow`, `brief_compose`, `pipeline_list`; `RecordRef` with the marks in their results |
| P11 | `theos_studio::codex` (search, read, list, graph) and the views for the Codex screen |
| P17 | `Marks::never` to withhold `never` truths from the editor's own prompts (G9) |
| P28 | Guides in `tools/guides/*.md` are served without an editor change |
| P49 | `--version` for `theos-mcp` |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| A sync per call on a large project | Calls slow down | P03 syncs by size and time; CE-12.1's budget holds for 500 records. A watcher can come later without changing a tool |
| Claude confirms a `never` truth in a draft | A guardrail breach in prose | The mark on every result and the server instructions say so; P07's propose path runs never-confirm on every proposal (G8) |
| `rmcp` changes its macros in a minor release | The build breaks on update | `Cargo.lock` pins it (G3); the tests cover every tool |
| Two Claude sessions on one project | One gets the cache, the other an index in memory | P05's fallback; both read the same files |
| A very long record or result | A large context for Claude | `limit` on search and list, the node cap on the graph; `codex_read` returns one record's text, which a client can read anyway |

## 8. Review Focus — inputs most likely to break it

- A `never` truth that appears only as a link target, a backlink source, a belief's `distorts`, or
  a ledger clue's truth: each must carry the mark.
- A duplicated id where only one of the records is a `never` truth or canon.
- A `target` or `status` with an invalid value (`Never`, `canon ` with a space in quotes): not a
  mark, because only valid values count; the record's findings show the problem.
- A record under `canon/formalism/` that fails to parse.
- An edit, a delete and a rename on disk between two calls; a change to `.theos/settings.toml`
  that changes a rule's severity.
- The cache held by another process at start, and released later.
- A start outside a project, then `theos.toml` created.
- A graph around a hub record (hundreds of backlinks), depth 3, and a cycle.
- A guide file with no heading, with CRLF, with non-UTF-8 bytes, and a `tools/guides` that is a
  file.
- Parameters: `limit` 0 and 101, both `id` and `path`, neither, an unknown key, a path with `\`
  or `..`.
- Anything on stdout that is not JSON-RPC: a panic message, a log line, a `println!` in a
  dependency.

## 9. Rulings (Claude's calls)

- Ruling: the queries live in `theos-studio` (`codex`), and `theos-mcp` only maps tools to them —
  P11's Codex needs the same queries, and P05 set this split — cost if wrong: one module moves.
- Ruling: results are `Json<View>` with an output schema, and the same JSON as text — clients
  that read `structuredContent` get the shape, others the text; one format to keep — cost if
  wrong: a text renderer per tool.
- Ruling: `locked` is a list of reasons (`canon`, `never`, `formalism`, `style`), not a boolean —
  P07 and the UI need to say why a record is locked — cost if wrong: one key's type.
- Ruling: only valid field values count for the marks — an invalid `status` is already a finding,
  and guessing a value would mark records the author did not lock — cost if wrong: a record with
  a misspelled `canon` is not flagged until it is fixed.
- Ruling: the server starts without a project and reports the reason in every call — Claude Code
  hides a server that fails at start behind "failed to connect" — cost if wrong: a few lines.
- Ruling: one call at a time on a current-thread runtime — the index is one SQLite connection, and
  calls are short — cost if wrong: a busy client waits for a slow call.
- Ruling: `codex_search` takes no kind filter — P03's search ranks across all records and a filter
  after the limit would drop hits; `codex_list` filters by kind — cost if wrong: one parameter
  later, with P03 filtering in SQL.
- Ruling: the graph caps at 200 nodes and depth 3 — a hub record at depth 3 can reach the whole
  project, which no context holds — cost if wrong: two constants.
- Ruling: `ledger_check` returns every `lore/` finding on the truths' and clues' paths — P04's
  registry has no reveal class, and a list in P48 would miss a new lore rule (directive 1);
  `inworld-vocab` on a clue is useful there too — cost if wrong: a filter on rule ids.
- Ruling: guides are read on every prompts call, and a missing folder is no guides — they change
  while a session runs, and most projects have none yet — cost if wrong: a cache.
- Ruling: the CE-08.1 smoke test gets `Stdio::null()` for stdin — the server now waits on stdin,
  and the test's assertion stays the same — cost if wrong: none; the test proves the same.
- Ruling: `codex_read` reads the record's file after the call's sync instead of a new text query
  on the index — the cache does not keep the whole text, and the sync just read the same file —
  cost if wrong: a query that stores and returns the text.
- Ruling: search matches carry the line, the matched words and the position, without markers —
  P03's markers are private noncharacters, and any visible marker could collide with record
  text — cost if wrong: one key.
- Ruling: `schemars` is a direct workspace dependency at rmcp's major version — the studio's views
  need `JsonSchema` and should not depend on `rmcp` — cost if wrong: a version bump in step with
  rmcp.
- Ruling: `rmcp`'s client is a dev-dependency for the binary tests — it speaks the protocol as
  Claude Code does, and directive 2 rules out a hand-written JSON-RPC client — cost if wrong: a
  heavier test build.

## 10. Left to the builder

- Module and type names inside the crates, as long as §5.5–§5.7 and §6 hold.
- The text of tool descriptions and the server instructions, within §5.2 and §5.8.
- Whether `Read` builds its lens from `EffectiveLens` directly or through a helper view.
- How the binary tests share the server start (a helper in `tests/common/`).
- The exact `tracing` events.
