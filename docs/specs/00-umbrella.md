# Theos content editor — umbrella spec

Status: accepted when this file is merged by Jann · version 3 · 10 Nov 2026 (version 1: 31 Oct 2026;
version 2, 5 Nov 2026, adds the world atlas, quests and art from lore: G19, P20–P28, slices S8 and
S9; version 3 moves the map generator to Rust in this repo, decision 0003: G1, G4, G11, G20, P08
rewritten, P09, P22 and P23 adjusted, P29, P30 and slice S10)
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
generative workflows (maps, concept art) built in, including notes that feed the next run; with an
atlas that ties places, people, enemies, spawns and quests to the maps; and with concept art and 2D
assets generated from the lore through prepared pipelines that agents can run.

## 2. Global constraints

- **G1** Repositories: `theos-editor` (this repo) holds the editor and the map generator
  (`theos-map-format`, `theos-maps`). `tower-of-theos` holds the content project, its map recipes
  and the blockout generator. `Theos-Engine` is never a dependency, except behind the cargo
  feature `engine`, which no phase here builds; the engine may depend on the map crates.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS are tested
  weekly, on demand (a label on the PR) and before every release (`docs/gate.md` §1).
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
- **G17** Branches, commits, PR titles, versions and tags follow `CONTRIBUTING.md`; releases
  follow `docs/release.md`. One product version for all crates; tags only from the pipeline.
- **G18** UI references are the HTML files in `docs/ui/screens/` (built only from
  `design/theos-ui/`); the PNG files next to them are previews rendered from that HTML.
- **G19** Positions on maps are fractions of a map version's frame (x east, y south, origin
  north-west); metres are derived from the frame. Pins and cameras live in the map record's body,
  never on the placed record (`docs/specs/data-model.md` §9.6–9.7).
- **G20** Map generation is deterministic: the same generator version, recipe, seed and inputs
  give the same package bytes and layers on every platform of G2. Canon maps are baked into
  packages (`docs/map-package.md`) and shipped, never regenerated in the game (decision 0003).

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
| P08 | Map format and terrain generator | theos-editor | S4 | P00 |
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
| P19 | Comments | theos-editor | S5 | P12, P13 |
| P20 | Content layout for world and art kinds | tower-of-theos | S8 | P01 |
| P21 | World kinds: enemies, spawns, quests | theos-editor | S8 | P07 |
| P22 | Map pins, cameras and frames | theos-editor | S8 | P09 |
| P23 | Map recipes and blockout generator | tower-of-theos | S8 | P08, P29 |
| P24 | Quest screen and flow editor | theos-editor | S8 | P12, P21 |
| P25 | Atlas | theos-editor | S8 | P16, P22, P24 |
| P26 | Style bible and briefs | theos-editor | S9 | P17, P21 |
| P27 | Asset pipelines | theos-editor | S9 | P16, P22, P26 |
| P28 | Workflow catalogue and asset guide | tower-of-theos | S9 | P20, P23 |
| P29 | Settlements, interiors and child maps | theos-editor | S8 | P08 |
| P30 | Indoor floors with stacked levels | theos-editor | S10 | P25, P29 |

Core track runs in order P00 → P02 → … → P07. P01, P08 and P10 can run in parallel sessions in
their own worktrees. UI phases start after P06. P20, P23 and P29 can also run in parallel sessions
once their dependencies are merged. P28 runs in a session linked to Jann's PC, where ComfyUI runs.

Slices: **S1** write lore now (P01, P10) · **S2** canon checks (P00, P02–P05) · **S3** propose and
review (P06, P07) · **S4** maps (P08, P09) · **S5** core UI (P11–P13, P19) · **S6** complete UI
(P14, P15) · **S7** pictures and places (P16–P18) · **S8** the world on the map (P20–P25, P29) ·
**S9** art from lore (P26–P28) · **S10** indoor floors (P30).

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

### P08 · Map format and terrain generator

The first part of the Rust map generator of decision 0003: the package format and the terrain
kind of `theos_maps` 0.3.0, which this generator replaces. Before the build, a gate PR adds the two
crates to the gate's crate lists and bans the std float methods (decision 0003 §12).

**CE-76 Map package format** (`theos-map-format`, `docs/map-package.md`)
- CE-76.1 A package written by `theos-map-format` and read back gives the same manifest, layers
  and features, and its bytes follow `docs/map-package.md` §1–§5 (checked against a fixture
  package).
- CE-76.2 A package whose `format_version` has an unknown major is refused with a message naming
  both versions; fields, entries, layers and feature kinds the reader does not know are ignored.
- CE-76.3 A synthetic package with two stacked levels round-trips with every layer and feature on
  its level; on a map with levels, a layer or feature without a level, or with a level the frame
  does not list, is rejected.
- CE-76.4 `theos-map-format` depends on no other crate of this workspace.

**CE-28 theos-maps terrain generator**
- CE-28.1 `describe` gives the name `theos-maps`, the generator's own version, `produces: ["map"]`
  and the recipes with their inputs: the built-in terrain recipes converted from `theos_maps`
  0.3.0 (`hearth-valley`, `hearth-valley-omega`, `fixture-small`) and the recipes of a given
  folder, where a recipe of the folder wins over a built-in one with the same id.
- CE-28.2 `run` reports a progress event per stage and ends with a manifest carrying the frame,
  the outputs `base`, `truth`, `inworld`, `geology`, `resources`, `features` and `package` (plus
  the extra renders a recipe asks for), and results with each constraint under the keys of
  `docs/generators.md` §3 and `pass`.
- CE-28.3 A recipe whose stage order leaves a stage without an input fails before the first stage
  runs, with a message naming the stage and the missing layer; an unknown stage or recipe input
  fails the same way.
- CE-28.4 Cancelling a run stops it at its next cancel point, within one second at 384², and
  leaves no files behind; a stage that panics ends the run as failed with the panic's message.
- CE-28.5 For each built-in recipe over seeds 1–8, the constraints pass at least as often as with
  `theos_maps` 0.3.0, and the medians of relief, lake share, river share, vegetation cover and
  median precipitation stay within the tolerances the phase spec sets around the 0.3.0 reference,
  recorded as fixtures.
- CE-28.6 Every feature has an `id`, `properties.kind` and `properties.type` as in
  `docs/map-package.md` §5; lakes carry `surface_m`, and rivers are lines with width, discharge
  and bed height; the `base` render lies exactly on the frame (a feature at a known position lands
  on the expected pixel).
- CE-28.7 The omega detuning stage of 0.3.0 is a stage of the generator; `hearth-valley-omega`
  writes the `omega` layer.

**CE-77 Determinism** (G20)
- CE-77.1 For fixed recipes and seeds, the hashes of every layer after every stage, of the
  `features` output and of the package bytes equal the golden hashes on Linux x64, Windows x64,
  and macOS on Apple silicon and Intel.
- CE-77.2 Golden hashes are kept per generator version: outputs that differ from the golden hashes
  of the current version fail a test, and the tool that records golden hashes refuses to overwrite
  those of an existing version.

### P09 · Generator layer and generator tools

**CE-29 Generator layer**
- CE-29.1 Generators registered in the settings (a part's default, the user's settings or
  `.theos/settings.toml`) are described and listed with their recipes.
- CE-29.2 A run over N seeds produces N candidate runs; keeping one writes a version record with
  outputs copied to the binaries folder and hashed.
- CE-29.3 Cancel stops a process generator's whole process tree on Windows, Linux and macOS, and a
  built-in generator's run at its next cancel point; partial outputs are discarded.
- CE-29.4 The built-in map generator runs in-process, each seed on a worker thread of its own,
  without Python, uv or network; a run that panics ends as a failed run and the editor keeps
  running.
- CE-29.5 `cargo xtask generator-check` passes for a conforming process generator and names the
  violation for one that breaks the protocol (both test fixtures).
- CE-29.6 Open notes of the parent are in the run request when the generator accepts notes.
- CE-29.7 Everything works with a fake generator used in tests, not only theos-maps.

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

### P19 · Comments

Comments on passages of a record, as designed in `docs/decisions/0002-comments-on-text.md`. Its
agent tools need the wording for G7 and CE-23.1 proposed in decision 0002 §10, which Jann settles
before this phase starts. The UI references for the margin, Review and Changes are added in this
phase's spec PR (G18).

**CE-51 Comment threads and anchors**
- CE-51.1 A record's threads are read from `.theos/comments/<id>.toml`; a comment file that does
  not parse is a finding with its path, and its record still opens.
- CE-51.2 After an edit made outside the editor, a passage that moved is found again, an edited
  passage attaches as changed, and a deleted passage is orphaned, never attached elsewhere.

**CE-52 Comment commands**
- CE-52.1 Opening, replying to, resolving, reopening and deleting a thread, and editing or
  withdrawing a comment, are commands with undo; none changes a record's bytes.
- CE-52.2 A comment written by an MCP process while the editor writes another to the same file
  is kept, and so is the editor's.
- CE-52.3 Renaming an id renames its comment file in the same command; deleting a record keeps
  its threads, which show as orphaned.

**CE-53 Comments over MCP**
- CE-53.1 `comment_list` and `comment_read` work over stdio; `comment_read` returns the passage
  with its lines and every comment with its author and time; threads on locked records are
  flagged and threads on `never` truths are marked, as in CE-18.3.
- CE-53.2 `comment_open` and `comment_reply` add comments under the session's name and change no
  record; agents cannot resolve, reopen, edit, withdraw or delete, and the comment tools of a
  paused session fail with "paused".
- CE-53.3 `comment_propose` creates a review entry linked to its thread; approving it resolves
  the thread, rejecting it keeps the thread open with the note.

**CE-54 Comments in the editor**
- CE-54.1 Selecting a passage and adding a comment opens a thread in the margin next to it;
  replying, resolving and reopening work there, and every comment shows its author and time.
- CE-54.2 A thread's passage follows typing, undo and outside changes; saving re-anchors exactly
  the threads whose passage moved or changed, and no others.
- CE-54.3 A thread whose passage is gone is listed as detached, with its quote, and can be
  re-attached or resolved.
- CE-54.4 An agent's comment or proposal appears in the open record without a reload and is
  marked as the agent's.
- CE-54.5 A review entry shows the threads its proposal resolves; Changes lists comment files
  apart from record changes.

### P20 · Content layout for world and art kinds (tower-of-theos)

The PR waits for Jann (content repo).

**CE-55 Layout and templates for the new kinds**
- CE-55.1 `enemies/` and `art/` exist with READMEs; quests live in `story/quests/` and spawns under
  their floor or location; the root README's table lists `enemy`, `spawn`, `quest` and `style`;
  `theos.toml` enables the part `atlas`.
- CE-55.2 One template and one VS Code snippet per new kind; a record made from each has valid
  front matter; the map template has empty `## Pins` and `## Cameras` sections.
- CE-55.3 `repo_checks` cover the new folders, templates and snippets on Ubuntu and Windows.

**CE-56 Guides for agents**
- CE-56.1 `tools/guides/place-on-the-atlas.md` and `tools/guides/write-a-quest.md` explain pins,
  cameras and quest steps as `docs/specs/data-model.md` §9.5–9.7 define them, each with one worked
  example marked as synthetic.

### P21 · World kinds: enemies, spawns, quests

CE-11.1 (P03) covers the kinds of the data model as they stood in umbrella version 1. Enemy, spawn
and quest get their schemas here, style in P26.

**CE-57 Enemies and spawns**
- CE-57.1 `enemy` and `spawn` have schemas registered by their editor parts; every corpus record of
  these kinds validates or reports exact errors.
- CE-57.2 A spawn's `of` accepts enemy, faction, people and npc and rejects any other kind; a
  `threat` outside 1–5 is an error.

**CE-58 Quest steps and flow**
- CE-58.1 A corpus quest parses into its steps with every attribute of data model §9.5; steps are
  addressable as `Q-012#2`; the body round-trips byte for byte.
- CE-58.2 An unreachable step, a path that never reaches `ends::`, a `next::`, `on::` or `after::`
  naming a missing step, and a loop without `repeat:: true` are each findings with their line.
- CE-58.3 never-confirm, no-overshoot and reveal-order run on every path of a quest: a fixture
  whose second branch overshoots a truth fails while its first branch alone would pass.
- CE-58.4 A step's `at` outside the quest's `where`, a revealed clue whose `where` lacks the step's
  place, and a dialogue whose speaker is neither in the step's `with` nor the giver are findings.
- CE-58.5 A step with `on::` outcomes and a `dialogue::` reports every outcome that has no section
  in that dialogue.
- CE-58.6 A step with `quest::` completes with that quest; quests that contain each other through
  `quest::` are a finding.

**CE-59 Quests over MCP**
- CE-59.1 `quest_flow` returns a quest's steps, arrows, paths and findings over stdio.
- CE-59.2 `quest_propose` creates a review entry; a proposal with blocking findings returns them
  and commits nothing.
- CE-59.3 A dialogue lists the quest steps that use it, a spawn the steps that fight it, and a
  place the quests that pass through it, all derived.

### P22 · Map pins, cameras and frames

**CE-60 Frames and features**
- CE-60.1 Keeping a map run writes the manifest's `frame` into the version record; a version
  without one is shown as unframed and refuses pins with a message.
- CE-60.2 The `features` output is read as GeoJSON with the `geojson` crate; feature ids, types and
  positions are available to queries; a missing or malformed layer is a warning, never an error.

**CE-61 Pins and cameras**
- CE-61.1 `## Pins` and `## Cameras` parse into their lines; every line round-trips byte for byte;
  a malformed line reports its line and column.
- CE-61.2 Findings: a location or spawn pinned on a map that is not for its place or a place above
  it; a record pinned twice on one map; a `feature` missing from the features layer; coordinates
  outside 0–1; a camera id used twice on one map; on a map whose frame lists levels, a pin without
  a `level` or with a level the frame does not list.
- CE-61.3 Pins of a child map appear on its parent map at the position its frame gives (fixture: a
  settlement inside a floor, correct within 0.5 m).
- CE-61.4 A map whose `placed_on` is not its picked version warns once; re-bind moves feature-bound
  pins to the nearest feature of the same type, keeps free pins and marks them for checking, in
  one command with undo.
- CE-61.5 The index answers "pins of a map", projected pins included, and "maps a record appears
  on"; 2 000 pins come back in under 100 ms on the CI runners, and changing one map updates only
  its entries.

**CE-62 Atlas over MCP**
- CE-62.1 `atlas_read` returns a map's pins, cameras, features and unplaced records, or the maps a
  record appears on; `atlas_place` proposes placing, moving and removing pins and adding cameras as
  review entries.
- CE-62.2 Everything in this phase works with the fake generator's maps, not only theos-maps.

### P23 · Map recipes and blockout generator (tower-of-theos)

The PR waits for Jann (content repo). The frame and features of map runs, which this phase built
in umbrella version 2, are built by the map generator in P08 and P29 (decision 0003).

**CE-63 Map recipes in the content project**
- CE-63.1 `tools/maps/recipes/` holds the project's map recipes as TOML, with a README;
  `tools/README.md` names the built-in map generator and this folder instead of `theos_maps`.
- CE-63.2 `repo_checks` fail for a recipe file that is not valid TOML, on Ubuntu and Windows.

**CE-64 theos_blockout**
- CE-64.1 `tools/theos_blockout` is a protocol-1 process generator run with uv and Python 3.12 on
  Windows and Linux, and on macOS or with the gap documented; `cargo xtask generator-check` passes
  for it.
- CE-64.2 From a settlement or interior map package (`.theosmap`, `docs/map-package.md`) it builds
  a scene whose buildings, streets and walls match the package's features within one map cell;
  from the `height` and `water` layers and the lake shapes it builds terrain with lake surfaces.
- CE-64.3 From a blockout script it builds the scene the script describes (synthetic fixture).
- CE-64.4 For each camera of its `cameras` input it writes clay, depth and canny renders at the
  requested size; the same input gives the same bytes on the same OS.
- CE-64.5 It runs without a GPU.

### P24 · Quest screen and flow editor

**CE-65 Quest screen** (`docs/ui/screens/14-quest.html`)
- CE-65.1 A quest shows as on the reference: a flow drawn from its body and laid out
  automatically, every finding on its step, and the generated paths beside it.
- CE-65.2 Adding a step, an arrow or an outcome on the diagram writes the body through the command
  layer with undo; untouched bytes stay identical.
- CE-65.3 A main-quest step with `quest::` opens that quest's flow as a sub-flow.
- CE-65.4 The selected step's attributes and prose are edited in the manuscript editor below the
  diagram.

### P25 · Atlas

Adds Atlas to the rail of every screen (`docs/architecture.md` §6).

**CE-66 Tower view** (`docs/ui/screens/13-atlas-tower.html`)
- CE-66.1 Bands and floors show as on the reference, with each floor's trial, side quests, counts,
  map and checks, and the main-quest thread across the floors; clicking a floor opens its map.
- CE-66.2 The tier filter hides what a player at that tier does not know yet.

**CE-67 Map views** (`docs/ui/screens/12-atlas.html`)
- CE-67.1 Floor, settlement and interior views show the picked version of the map for that place,
  with a breadcrumb back up; the layers Places, NPCs, Spawns, Quests, Lore and Concepts switch on
  and off.
- CE-67.2 The tier filter hides quests, clues and places not revealed at the chosen tier.
- CE-67.3 Clicking a place opens its panel with lore, people, quests, nearby spawns, concepts and
  its own map; "Open map" drills down.
- CE-67.4 Dragging a record from Unplaced places it and dragging a pin moves it; both are commands
  with undo.
- CE-67.5 Clicking a quest highlights its route, branches drawn as forks, and opens its flow in a
  drawer; hovering a step highlights its pin, and the other way round.
- CE-67.6 Placements proposed by an agent show as proposed until they are reviewed.

### P26 · Style bible and briefs

**CE-68 Style records**
- CE-68.1 `style` has a schema; the effective style of a concept resolves through `extends` and
  shows where each section comes from.
- CE-68.2 A change to a locked style is a high-significance review entry; no MCP tool locks or
  unlocks a style.
- CE-68.3 `refs` are binary references, checked like every other (CE-15.4).

**CE-69 Briefs**
- CE-69.1 A brief for a record, concept type and tier is built from a `template` with
  `purpose: prompt` out of the record and its `## Look`, its place chain, the people living there,
  the effective style and the concept's open notes (data model §9.9).
- CE-69.2 A brief for T1 holds nothing revealed only after T1 (fixture), and text of a `never`
  truth is never in a brief (tested; G9).
- CE-69.3 `brief_compose` returns over MCP the same brief the editor shows.
- CE-69.4 A prompt drafted by the LLM client is marked AI, stays editable, and is stored in the
  version's `inputs` when a run starts.

### P27 · Asset pipelines

**CE-70 Pipelines and stages**
- CE-70.1 Pipelines in `tools/comfy/pipelines/*.toml` are listed with their stages; a stage naming
  an unknown workflow, slot or slot source is a finding with its line.
- CE-70.2 Running a stage produces concept versions whose `from` is the previous stage's pick and
  whose `inputs` name the pipeline and the stage; a stage with `pick = "auto"` runs on without a
  pick.
- CE-70.3 The slot sources of `docs/generators.md` §8 resolve; one that does not stops the run
  before it starts and names the source.
- CE-70.4 A workflow whose nodes or models ComfyUI lacks shows as not ready, with the missing names,
  from `/object_info` and `/models` (recorded responses in tests).
- CE-70.5 A pipeline refuses a concept whose `for` lacks a record it needs, and sets the concept's
  `type`.

**CE-71 Generating in the editor** (`docs/ui/screens/15-generate.html`)
- CE-71.1 "Generate concept" on a record or an atlas place opens the screen as on the reference:
  family, pipeline, brief, prompt, the cameras of the place's maps and the readiness checks.
- CE-71.2 The Concepts screen shows a concept's pipeline as its stage strip, as on
  `docs/ui/screens/06-concepts.html`.

**CE-72 Pipelines and guides over MCP**
- CE-72.1 `pipeline_list` and `pipeline_run` work over stdio; Claude runs a whole pipeline for a
  place with a pick per stage, and its picks land as review entries.
- CE-72.2 The guides in `tower-of-theos/tools/guides/*.md` are served as MCP prompts and listed by
  `project_info`.

### P28 · Workflow catalogue and asset guide (tower-of-theos)

The PR waits for Jann (content repo). The phase runs in a session linked to Jann's PC, where
ComfyUI runs. The PR carries one proof sheet per asset family (the outputs of one full pipeline
run, in the binaries folder); Jann's judgement of them is a review item of the PR, not an AC.

**CE-73 Workflow catalogue**
- CE-73.1 The workflows `klein-compose`, `zimage-cn-compose`, `klein-ref-look`, `krea-finish`,
  `zimage-4k`, `crop-detail` and `color-match` exist in API format with sidecars as in
  `docs/generators.md` §7.
- CE-73.2 Workflows for the families characters, NPCs in a scene, creatures, items and clothes, and
  in-world maps exist the same way.
- CE-73.3 Every sidecar slot maps to an input of an existing node, checked against a recorded
  `/object_info`, and every model is listed with its file name, folder and source.
- CE-73.4 Every sidecar records the peak VRAM measured at the default size, and none exceeds 16 GB.

**CE-74 Pipelines and the asset guide**
- CE-74.1 One pipeline per asset family of the asset guide, each valid against
  `docs/generators.md` §8.
- CE-74.2 `tools/guides/asset-workflow.md` says which pipeline serves which family, how picks and
  notes drive the next stage, and what fits in 16 GB.

### P29 · Settlements, interiors and child maps

The rest of the port of `theos_maps` 0.3.0 into `theos-maps` (decision 0003): settlement and
interior maps, one level each, and child maps made inside a parent map.

**CE-78 theos-maps settlements, interiors and child maps**
- CE-78.1 The settlement and interior kinds of 0.3.0 run from built-in recipes (`choir-hall-ruin`
  and a settlement recipe), with the renders `base`, `plan`, `blueprint` and `graph`, and packages
  whose layers and features follow `docs/map-package.md` §4.4 and §5 (buildings, streets, walls,
  gates, landmarks; rooms as polygons, doors, connections, the critical route).
- CE-78.2 A child map runs from a parent package (`from`) and one of its features (an input of
  type `feature`: a settlement or a ruin); its frame carries `origin_m` in the parent's frame and
  `parent` with the parent package's hash and the feature id; the same parent, feature, seed and
  inputs give the same child.
- CE-78.3 For seeds 1–8 of each built-in interior recipe, every interior passes the playability
  analysis of 0.3.0 (every room carved and reachable, lock and key solvable with one-way drops);
  settlement summaries stay within the tolerances the phase spec sets around the 0.3.0 reference.
- CE-78.4 The golden hashes of CE-77.1 cover a settlement, an interior and a child map.

### P30 · Indoor floors with stacked levels

Floors that are indoors, with storeys on top of each other (decision 0003 §7). The UI reference
for the level switcher is added in this phase's spec PR (G18).

**CE-79 Indoor floors**
- CE-79.1 An indoor-floor recipe builds an interior up to 2 km across with at least two stacked
  levels: the frame lists the levels, every layer and feature carries its level, and stairs,
  ladders, lifts and drops connect levels.
- CE-79.2 The playability analysis covers paths across levels: every room is reachable from the
  start, lock and key are solvable, and drops go one way only.
- CE-79.3 The Atlas shows a map with levels one level at a time, with a level switcher; a pin
  shows on its own level only.
- CE-79.4 The golden hashes of CE-77.1 cover an indoor floor with levels.

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
| CE-19, CE-27, CE-31 | theos-design plugin: read through MCP (after P05), propose through MCP (after P07), maps through the generator tools (after P09), quests, the atlas and pipelines through their tools and the guides in `tools/guides/` (after P21, P22, P27). |
| CE-75 | Move the style bible (`style.yaml` v2 and its reference crops) from Drive into `art/` style records (after P20). |

## 5. Later (not planned)

Dragging a clue to another floor; graph paths between two records; who-wrote-what view; keymap
editor; Assets, Layout and AI presets (engine part); bulk approve; the game engine as a map
generator. From version 2: tuning fields for enemies and spawns (stats, spawn tables, timers,
loot) with the engine part; exporting pins, spawns and quests to the engine (as GeoJSON beside
the map package, `docs/map-package.md` §9); NPC schedules and NPCs that move; polygon areas on
maps; tiled map images beyond 4K; 3D assets (meshes, textures); a LoRA per main character if faces
drift between images; runtime dialogue built from quest step context. From version 3: refining
baked maps at a finer scale in the engine; generating dungeons or detail from a seed in the game;
a `theos gen` subcommand that runs generators without the editor; the blockout generator in Rust.
