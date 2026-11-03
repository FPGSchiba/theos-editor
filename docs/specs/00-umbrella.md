# Theos content editor — umbrella spec

Status: accepted when this file is merged by Jann · version 1 · 31 Oct 2026
Owner of architecture: Jann · Built by: Claude Code with superpowers

This spec defines the whole build as phases. Each phase runs one superpowers cycle:
**phase spec → plan → subagent-driven build → final review loop → PR**
(`.claude/skills/theos-phase/SKILL.md`).

**Precedence:** this file > `docs/architecture.md`, `docs/specs/data-model.md`,
`docs/generators.md`, `docs/gate.md` > phase spec (`docs/specs/phases/`) > plan (`docs/plans/`).
A phase spec adds detail; it never changes an acceptance criterion (AC) defined here. Changing an
AC is an edit of this file and needs Jann's approval.

**Readable copies** of vision, architecture, data model and plan live in Confluence (TOT). Work
sessions cannot reach Confluence; this repo is the source of truth for building.

## 1. Goal

A working content editor for Tower of Theos, without the engine: lore, world, people, story,
concepts and maps as Markdown records in git; checked against canon on every change; edited by
Jann in a desktop app and by Claude through MCP using the same validated commands; with
generative workflows (maps, concept art) built in, including notes that feed the next run.

## 2. Global constraints

- **G1** Repositories: `theos-editor` (this repo) holds the editor. `tower-of-theos` holds the
  content project and the map toolset. `Theos-Engine` is never a dependency, except behind the
  cargo feature `engine`, which no phase here builds.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS are tested
  weekly, on demand (a label on the PR) and before every release (`docs/gate.md` §1).
- **G3** Rust: stable toolchain pinned in `rust-toolchain.toml`, edition 2024, `unsafe_code`
  forbidden, `Cargo.lock` committed, all builds `--locked`.
- **G4** Crates: `theos-content`, `theos-vcs`, `theos-studio` (libs); `theos`, `theos-mcp`,
  `theos-editor` (bins); `xtask`. Paths `crates/<name>`, `xtask/`. UI in `editor-ui/`
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
- **G11** Generators only through `docs/generators.md`; no generator-specific field in the data
  model; the UI works for any generator that produces a kind.
- **G12** Network only to ComfyUI and the configured LLM endpoint, both opt-in; no telemetry.
- **G13** UI uses `design/theos-ui/` and follows `docs/ui/`; shortcuts use Ctrl on Windows and
  Linux and Cmd on macOS, from one keymap.
- **G14** Settings in three layers (part default → user → project); secrets in the OS
  credential store.
- **G15** Every AC below has at least one test tagged with its id (`docs/gate.md` §4).
- **G16** Errors in library crates are typed (`thiserror`); `anyhow` only in binaries. Logging
  with `tracing`.
- **G17** Branches, commits, PR titles, versions and tags follow `CONTRIBUTING.md`; releases
  follow `docs/release.md`. One product version for all crates; tags only from the pipeline.
- **G18** UI references are the HTML files in `docs/ui/screens/` (built only from
  `design/theos-ui/`); the PNG files next to them are previews rendered from that HTML.

## 3. Phases

Slices say what Jann can do once their phases are merged; Jira tracks slices.

| Phase | Name | Repo | Slice | Depends on |
|---|---|---|---|---|
| P00 | Foundation and gate | theos-editor | S2 | – |
| P01 | Content layout and templates | tower-of-theos | S1 | – |
| P02 | Records and grammars | theos-editor | S2 | P00 |
| P03 | Kinds, index, search, lenses | theos-editor | S2 | P02 |
| P04 | Validation, lore rules, ledger, formalism mirror | theos-editor | S2 | P03 |
| P05 | CLI, hook, releases, MCP read | theos-editor | S2 | P04 |
| P06 | Command layer and git | theos-editor | S3 | P04 |
| P07 | Agent sessions, review, MCP propose | theos-editor | S3 | P05, P06 |
| P08 | Map toolset as a generator | tower-of-theos | S4 | P01 |
| P09 | Generator layer and generator tools | theos-editor | S4 | P07, P08 |
| P10 | Prose editor research | theos-editor | S1 | – |
| P11 | Editor shell and Codex | theos-editor | S5 | P06 |
| P12 | Manuscript editor | theos-editor | S5 | P10, P11 |
| P13 | Changes and Review screens | theos-editor | S5 | P07, P11 |
| P14 | Start and Settings | theos-editor | S6 | P11 |
| P15 | Lenses, Scripts, Graph | theos-editor | S6 | P04, P12 |
| P16 | Generative screens: Concepts, ComfyUI, Maps | theos-editor | S7 | P09, P12 |
| P17 | AI drafts and semantic search | theos-editor | S7 | P04, P15 |
| P18 | Installer, first run, Code preset | theos-editor | S7 | P14 |

Core track runs in order P00 → P02 → … → P07. P01, P08 and P10 can run in parallel sessions in
their own worktrees. UI phases start after P06.

Slices: **S1** write lore now (P01, P10) · **S2** canon checks (P00, P02–P05) · **S3** propose and
review (P06, P07) · **S4** maps (P08, P09) · **S5** core UI (P11–P13) · **S6** complete UI (P14,
P15) · **S7** pictures and places (P16–P18).

---

### P00 · Foundation and gate

The repo skeleton and the gate of `docs/gate.md`. This phase changes protected paths, so its PR
waits for Jann.

**CE-08 Workspace, CI and gate**
- CE-08.1 The workspace with all crates of G4 (empty, documented) builds and tests in CI on
  Linux and Windows.
- CE-08.2 CI fails when a crate depends on an engine crate without feature `engine`.
- CE-08.3 Every check of `docs/gate.md` §1 (except `ui`) exists and is a required status check on
  `main`; branch protection and repo settings match `docs/gate.md` §6.
- CE-08.4 `cargo xtask gate` gives the same verdicts as CI on Windows and Linux.
- CE-08.5 `approval` fails for a PR that touches a protected path without the label `approved`
  and passes once it is added; CI adds `needs-jann` to such PRs.
- CE-08.6 `ac-trace` fails for a phase AC without a tagged test and for a tag naming an unknown AC.
- CE-08.7 `pr-body` fails for a description missing a template section.
- CE-08.8 `coverage` fails when a crate is below its floor or drops more than allowed below `main`.
- CE-08.9 `pr-title` fails for a title that is not a Conventional Commit ending with `(PNN)`,
  `(gate)`, `(docs)` or a release title.
- CE-08.10 `commit-msg` fails when any commit of the PR is not a Conventional Commit.
- CE-08.11 The workspace has one version in `[workspace.package]` that every crate inherits.
- CE-08.12 A scheduled workflow runs `clippy` and `test` on macOS weekly and on demand.

### P01 · Content layout and templates (tower-of-theos)

The PR waits for Jann (content repo).

**CE-01 Content layout**
- CE-01.1 `theos.toml`, `.theos/` and the folder tree of `docs/architecture.md` §2 exist, each
  top-level folder with a README.
- CE-01.2 `.gitattributes` forces LF for `md`, `toml`, `yaml`, `json`, `py`.
- CE-01.3 `.theos/cache` is ignored.
- CE-01.4 The root README says where each kind lives, that system canon stays in Confluence,
  that binaries live in a separate folder the user syncs, and that the clone must be on a local disk.

**CE-03 Record templates**
- CE-03.1 One template per kind in `templates/records/`; a record made from it in Obsidian has
  valid front matter.
- CE-03.2 Obsidian settings in `.obsidian/` point at the templates and treat `[[id]]` links as
  file links; VS Code snippets exist for every kind.

### P02 · Records and grammars

**CE-09 Record model and front-matter patcher**
- CE-09.1 Every record in the fixture corpus round-trips byte for byte.
- CE-09.2 Setting one field changes only that field's lines.
- CE-09.3 Anchors, aliases, tags, multi-line scalars, flow maps and deeper nesting are rejected
  with line and column.
- CE-09.4 CRLF input is read; output is LF.

**CE-10 Body grammars**
- CE-10.1 S-004 of the corpus parses into its beats with their attribute links.
- CE-10.2 Voice blocks, design notes and note callouts parse with their title tokens; note states
  `open`, `resolved by [[id]]` and `dropped` are recognised.
- CE-10.3 A malformed beat attribute, callout title or wikilink reports its line.
- CE-10.4 Bodies round-trip byte for byte.

The fixture corpus starts with the examples of `docs/specs/data-model.md` and synthetic records;
real records are added once the lore import exists.

### P03 · Kinds, index, search, lenses

**CE-11 Kinds and schemas**
- CE-11.1 Every kind of the data model has a schema; every corpus record validates or reports
  exact errors.
- CE-11.2 A link to a kind not allowed for that field is an error.
- CE-11.3 A `schema_version` mismatch gives a clear message.
- CE-11.4 Unknown fields warn.
- CE-11.5 Kinds are registered by editor parts through the registry, not hard-coded in one list.

**CE-12 Index**
- CE-12.1 A full index of 500 records takes under one second on the CI runners.
- CE-12.2 Changing one file updates that record and its backlinks only.
- CE-12.3 Duplicate ids, dangling links and file names that differ from ids are findings with paths.
- CE-12.4 Deleting the cache rebuilds the same index.

**CE-13 Search and effective lenses**
- CE-13.1 Full-text search returns ranked records with the matching text.
- CE-13.2 An npc's effective lens shows which layer each value comes from.
- CE-13.3 A child overriding a parent value shows as an override.

### P04 · Validation, lore rules, ledger, formalism mirror

**CE-14 Validator core**
- CE-14.1 A setting that lowers a guardrail is rejected with a message.
- CE-14.2 Findings carry rule, severity, file and line.
- CE-14.3 Validating one record also validates the neighbours it affects.

**CE-15 Lore rules and ledger**
- CE-15.1 never-confirm, no-overshoot, inworld-vocab, reveal-order, requires and three-clue each
  have a passing and a failing fixture.
- CE-15.2 An allowlist entry without a reason is an error.
- CE-15.3 The ledger is built from clue records into the cache and printed by `theos ledger`.
- CE-15.4 A missing binary or a hash mismatch is a warning.
- CE-15.5 A note title with an unknown state, or `addresses` naming a note that does not exist, is
  an error.

**CE-16 Shared Formalism mirror**
- CE-16.1 `canon/formalism/_source.json` (written once by a private session from Confluence)
  generates the records `E1`…`E18`.
- CE-16.2 A record citing `E7` resolves.
- CE-16.3 A hand edit of a mirror record fails the check.
- CE-16.4 Regenerating is idempotent.

### P05 · CLI, hook, releases, MCP read

**CE-17 CLI and hook**
- CE-17.1 `theos check | ledger | new | schema | hook install` with `--json` output.
- CE-17.2 The hook refuses a commit with a never-confirm violation and checks the staged snapshot.
- CE-17.3 A GitHub Action for tower-of-theos runs the released `theos check` and shows findings.

**CE-08R Release pipeline** (`docs/release.md`)
- CE-08R.1 Pushes to `main` keep one release PR up to date with the next version and a
  `CHANGELOG.md` section generated from the Conventional Commits since the last tag.
- CE-08R.2 Merging the release PR creates exactly one annotated tag `vX.Y.Z` on `main`; all
  crates carry that version; nothing is published to crates.io.
- CE-08R.3 The tag builds `theos` and `theos-mcp` for Windows, Linux and macOS (universal
  binary) with `--locked` and publishes them with `SHA256SUMS` and build provenance in a GitHub
  Release whose notes are the changelog section and the macOS first-start steps.
- CE-08R.6 The release workflow runs the full test suite on Windows, Linux and macOS first and
  creates no release if any platform fails.
- CE-08R.4 A tag with `-rc.N` produces a GitHub pre-release.
- CE-08R.5 `theos --version` and `theos-mcp --version` print the version, the commit, the data
  model `schema_version` and the generator protocol version.

**CE-18 MCP read tools**
- CE-18.1 `project_info`, `codex_search`, `codex_read`, `codex_list`, `codex_graph`,
  `ledger_check` work over stdio from Claude Code.
- CE-18.2 Claude answers "what does Mother Ilse believe" from records alone.
- CE-18.3 Locked records are readable and flagged; truths with target `never` are marked in every
  result.

### P06 · Command layer and git

**CE-20 Command layer**
- CE-20.1 A blocking finding rejects the command and leaves files untouched.
- CE-20.2 Undo restores the bytes.
- CE-20.3 Renaming an id updates every inbound link in one command.

**CE-21 Git read**
- CE-21.1 A renamed record keeps its history.
- CE-21.2 Field diff shows old and new per field; prose diff is word-level.
- CE-21.3 A missing or too-old git gives a clear message.

**CE-22 Git write and repo lock**
- CE-22.1 Commit is blocked on errors in the staged snapshot and allowed with warnings.
- CE-22.2 The repo lock keeps two writers apart across processes.
- CE-22.3 Push errors show git's message; credentials only through the system helper.

### P07 · Agent sessions, review, MCP propose

**CE-23 Agent sessions**
- CE-23.1 Two parallel sessions never touch the human working tree or each other.
- CE-23.2 Ids reserved by two sessions never collide.
- CE-23.3 Propose calls of a paused session fail with "paused".
- CE-23.4 Closing a session removes its worktree and keeps its branch until reviewed.

**CE-24 Review core**
- CE-24.1 Approval aborts when the merged result has errors.
- CE-24.2 A change to a canon truth or a `never` truth is high significance.
- CE-24.3 Approval waits and names the record when the human has uncommitted edits to it.
- CE-24.4 Reject keeps the branch and marks the entry.

**CE-25 Review CLI**
- CE-25.1 `theos review list | show | approve | reject --note` approves a proposal end to end.
- CE-25.2 `show` prints the word diff and findings.

**CE-26 MCP propose tools**
- CE-26.1 `codex_propose`, `script_propose`, `review_list` exist; every proposal appears in
  `theos review list`.
- CE-26.2 A proposal touching a locked record fails with the reason.
- CE-26.3 A proposal with blocking findings returns them and commits nothing.

### P08 · Map toolset as a generator (tower-of-theos)

The PR waits for Jann (content repo).

**CE-28 theos_maps as a protocol-1 generator**
- CE-28.1 `tools/theos_maps` is a package (pyproject) runnable with `uv` and Python 3.12 on
  Windows and Linux.
- CE-28.2 `python -m theos_maps.gen describe` prints a valid protocol-1 description.
- CE-28.3 `run` emits a progress event per stage and a manifest with outputs (renders,
  heightfield, engine package) and results (constraint values, pass).
- CE-28.4 A determinism fixture gives identical output hashes on Windows and Linux, or the
  difference is documented with a tolerance.
- CE-28.5 The old entry point `python -m theos_maps.run` keeps working.
- CE-28.6 The toolset installs and runs on macOS (Apple silicon and Intel), or the gap is
  documented with a workaround. (fastscapelib 0.3.0 has conda-forge builds for macOS; whether
  PyPI wheels exist is checked in this phase.)

### P09 · Generator layer and generator tools

**CE-29 Generator layer**
- CE-29.1 Generators registered in `.theos/settings.toml` are described and listed with their recipes.
- CE-29.2 A run over N seeds produces N candidate runs; keeping one writes a version record with
  outputs copied to the binaries folder and hashed.
- CE-29.3 Cancel stops the whole process tree on Windows, Linux and macOS; partial outputs are
  discarded.
- CE-29.4 First use of the maps generator creates the uv venv with Python 3.12 and leaves the
  system Python untouched.
- CE-29.5 `cargo xtask generator-check` passes for theos_maps and names the violation for a
  generator that breaks the protocol.
- CE-29.6 Open notes of the parent are in the run request when the generator accepts notes.
- CE-29.7 Everything works with a fake generator used in tests, not only theos_maps.

**CE-30 Generator and note tools over MCP**
- CE-30.1 `gen_catalogue` and `gen_runs` read; `gen_run`, `gen_pick`, `note_add`, `note_resolve`
  propose.
- CE-30.2 Claude runs four seeds of a recipe and reads the results per seed.
- CE-30.3 Picks and notes from agents land as review entries.

### P10 · Prose editor research

Output: a decision record in `docs/decisions/` (protected, Jann approves).

**CE-06 Choose the prose editor component**
- CE-06.1 Comparison of CodeMirror 6, ProseMirror/Tiptap, Milkdown, Lexical and Monaco on:
  byte-stable Markdown, front matter untouched, inline link chips, callouts with their own look,
  beat headings with attribute lines, spell check and IME, long documents, Svelte 5 integration,
  licence, maintenance. Sources cited.
- CE-06.2 Prototypes of the top two load and save the corpus; saved bytes are compared and
  reported.
- CE-06.3 A recommendation Jann can accept or reject.

### P11 · Editor shell and Codex

From this phase the gate includes `ui` (added by an approved gate PR inside this phase).

**CE-32 Editor shell**
- CE-32.1 The app opens a project and shows the rail of `docs/architecture.md` §6.
- CE-32.2 Shortcuts (Ctrl on Windows and Linux, Cmd on macOS) and a command palette.
- CE-32.3 Builds with and without the `engine` feature (the feature is empty).
- CE-32.4 IPC types for the UI are generated from Rust; a mismatch fails the build.

**CE-33 Codex view**
- CE-33.1 A floor shows as on `docs/ui/screens/01-main.html`.
- CE-33.2 Backlinks are derived.
- CE-33.3 Search jumps to a record.

### P12 · Manuscript editor

**CE-34 Manuscript editor core** (on the component chosen in P10)
- CE-34.1 Saving leaves untouched bytes identical.
- CE-34.2 An external change reloads a clean record and offers a merge for a dirty one.
- CE-34.3 Field undo hands over to command undo on save.

**CE-35 Link chips, autocomplete, callouts**
- CE-35.1 `[[` suggests ids of the kinds allowed at that place.
- CE-35.2 A dangling link shows as an error.
- CE-35.3 Voice blocks render in the in-world style and flag developer words inline; note and
  design callouts render with their own look.

### P13 · Changes and Review screens

**CE-36 Changes** (`docs/ui/screens/08-git.html`)
- CE-36.1 Commit is blocked on errors in the staged snapshot.
- CE-36.2 History follows renames.
- CE-36.3 Pull conflicts are listed by record and handed to the merge tool.

**CE-37 Review** (`docs/ui/screens/07-review.html`)
- CE-37.1 Approve and reject behave exactly as the CLI.
- CE-37.2 Pausing a session stops its propose calls.
- CE-37.3 There is no bulk approve.

### P14 · Start and Settings

**CE-38 Start screen** (`docs/ui/screens/09-start.html`)
- CE-38.1 Recent and pinned projects; a missing project shows as missing.
- CE-38.2 A new content project creates `theos.toml` and the tree.
- CE-38.3 "From an existing folder" adds `theos.toml` to a folder.
- CE-38.4 The game kind is shown and disabled.

**CE-39 Settings** (`docs/ui/screens/10-settings.html`)
- CE-39.1 Each value shows its layer.
- CE-39.2 Guardrails have no Off.
- CE-39.3 Secrets go to the OS credential store.
- CE-39.4 The binaries folder is set per machine; generators are listed with their status.

### P15 · Lenses, Scripts, Graph

**CE-40 Lenses** (`docs/ui/screens/02-lens.html`)
- CE-40.1 The effective lens shows each value's origin.
- CE-40.2 What a holder believes is derived, not stored.

**CE-41 Scripts** (`docs/ui/screens/04-script.html`)
- CE-41.1 Beats reference clues through `reveals::`.
- CE-41.2 A never-confirm finding shows on the beat that causes it.

**CE-42 Graph** (`docs/ui/screens/05-graph.html`)
- CE-42.1 Links around a record with kind filter and depth.
- CE-42.2 Colours come from Theos UI tokens.

### P16 · Generative screens: Concepts, ComfyUI, Maps

**CE-43 Concepts** (`docs/ui/screens/06-concepts.html`)
- CE-43.1 Versions with lineage, rating keep/kill/mix, pick per state.
- CE-43.2 Notes: add to the concept or a version, see open notes beside the versions, resolve a
  note from a later version.
- CE-43.3 Images load from the binaries folder; a missing or changed file shows as such.

**CE-44 ComfyUI adapter**
- CE-44.1 Workflow templates with named slots in `tower-of-theos/tools/comfy/workflows/` are
  listed as recipes.
- CE-44.2 A run produces concept versions with lineage and outputs in the binaries folder.
- CE-44.3 ComfyUI being down gives a clear message within 5 seconds, not a hang.
- CE-44.4 The same runs are available through the `gen_*` MCP tools.

**CE-45 Maps** (`docs/ui/screens/11-maps.html`)
- CE-45.1 Run seeds with live progress and cancel.
- CE-45.2 Results per seed side by side; pick marks the canon version for its map.
- CE-45.3 Notes as in Concepts.
- CE-45.4 The screen works for any generator that produces maps (tested with the fake generator).

### P17 · AI drafts and semantic search

**CE-46 LLM client and compile preview**
- CE-46.1 The prompt carries only what the speaking lens may know.
- CE-46.2 Text of `never` truths is never in a prompt (tested).
- CE-46.3 Every lore rule runs on the draft before it is shown.
- CE-46.4 Drafts carry the AI marker.

**CE-47 Suggested commit messages**
- CE-47.1 A suggestion is a marked draft the human edits; nothing is committed without them.

**CE-48 Semantic search and near-duplicate warning**
- CE-48.1 Embeddings are cached and rebuilt on change.
- CE-48.2 A new truth or belief close to an existing one warns with the match.

### P18 · Installer, first run, Code preset

**CE-49 Installer and first run**
- CE-49.1 The release workflow attaches a Windows installer and a universal macOS disk image
  (Tauri bundler) to every release; the app's version is the workspace version.
- CE-49.2 First run checks git, uv, the binaries folder and ComfyUI, and says what is missing.

**CE-50 Code preset**
- CE-50.1 Plain files open in a code view.
- CE-50.2 "Open in VS Code" for the current file.

---

## 4. Work outside the build

Done by Jann with Claude in a private session (Confluence and Jira are not reachable from the
work sessions). Each item lands in tower-of-theos or Confluence, not in this repo.

| Item | What |
|---|---|
| CE-02 | Content Data Model — done (`docs/specs/data-model.md`). |
| CE-04 | Import the 12 lore pages as 12 topic records; extract first truths, beliefs and clues as proposals; frozen banners on the Confluence pages. |
| CE-05 | Split the false Seal into atomic truths with reveal targets; update the plugin's canon rule. |
| CE-07 | Decision pages in Confluence for the decision log; Workspace Crate Architecture page. |
| CE-16S | Write `canon/formalism/_source.json` from Shared Formalism (needed by P04). |
| CE-19, CE-27, CE-31 | theos-design plugin: read through MCP (after P05), propose through MCP (after P07), maps through the generator tools (after P09). |

## 5. Later (not planned)

Dragging a clue to another floor; graph paths between two records; who-wrote-what view; keymap
editor; Assets, Layout and AI presets (engine part); bulk approve; the game engine as a map
generator.
