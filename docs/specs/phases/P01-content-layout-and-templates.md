# P01 · Content layout and templates — phase spec

Status: waiting for Jann's approval · 31 Oct 2026 · repo of the build: `tower-of-theos`
Source: `docs/specs/00-umbrella.md` §3 P01. Scope and acceptance criteria come from there; this
spec decides how.

## 1. Goal

Slice S1 is "write lore now". After P01, Jann can open `tower-of-theos` in Obsidian or VS Code
and write records that already follow `docs/specs/data-model.md`, months before the editor
exists. Everything P01 creates — the folder tree, `theos.toml`, the templates — is what the
later phases read: the index (P03), the checks (P04), `theos check` and its GitHub Action (P05),
the map toolset (P08) and the Start screen (P14).

## 2. Acceptance criteria (verbatim from the umbrella)

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

## 3. Scope

**Goals.** A layout that matches the architecture exactly; templates that produce valid records
with as little typing as the tools allow; tool settings that never damage restricted YAML;
everything checked by tests on Windows and Linux.

**Non-goals.** No content records — not even `world/tower.md`; content comes from the lore import
(CE-04) or from Jann. No map toolset (P08), no generator registration (P09), no ComfyUI workflows
(P16), no `theos check` action (P05). No Obsidian community plugins, themes or CSS snippets. No
`.theos/sessions/` (P07 decides whether it is tracked).

**Anti-goals** (would make P01 a failure even if every test passes):
- a tool setting that rewrites front matter into something the restricted YAML rejects, or drops
  comments, without Jann doing it on purpose;
- a template default that fails open on a guardrail (for example a truth that silently becomes
  revealable because a placeholder looked like a real value);
- an easy path for binaries into git (G10);
- opening the vault in Obsidian creating diffs in tracked files;
- invented lore in templates, READMEs or test data.

## 4. Design

### 4.1 Tree

```text
tower-of-theos/
  README.md                     root README (§4.4)
  theos.toml                    §4.2
  .gitattributes  .gitignore    §4.3
  .theos/        README.md · settings.toml
  world/         README.md
  people/        README.md
  things/        README.md
  lore/          README.md · topics/ · truths/ · beliefs/ · clues/
  story/         README.md · scripts/
  concepts/      README.md
  maps/          README.md
  templates/     README.md · records/ (the 20 record templates `<kind>.template.md`, §4.5)
  dialogue/      README.md
  canon/         README.md · formalism/
  tools/         README.md · comfy/workflows/ · repo_checks/ (§4.8)
  .obsidian/     app.json · core-plugins.json · templates.json (§4.6)
  .vscode/       theos.code-snippets (§4.7)
  .github/workflows/checks.yml  (§4.8)
```

- "Top-level folder" in CE-01.1 means the folders of the architecture tree: `.theos`, `world`,
  `people`, `things`, `lore`, `story`, `concepts`, `maps`, `templates`, `dialogue`, `canon`,
  `tools`. Each has a `README.md`. Tool folders (`.obsidian`, `.vscode`, `.github`) have none.
- Empty leaf folders (`lore/topics/`, `lore/truths/`, `lore/beliefs/`, `lore/clues/`,
  `story/scripts/`, `canon/formalism/`, `tools/comfy/workflows/`) hold an empty `.gitkeep`, so
  git keeps them. `tools/theos_maps/` is left to P08.

### 4.2 `theos.toml` and `.theos/`

`theos.toml` is exactly the example of `docs/specs/data-model.md` §13. `.theos/settings.toml` is
tracked and holds only a comment saying what goes there (generators, from P09); it must parse as
TOML. `.theos/README.md` explains `settings.toml` (tracked), `sessions/` (agent sessions, P07)
and `cache/` (ignored, rebuilt, safe to delete).

### 4.3 `.gitattributes` and `.gitignore`

- `.gitattributes`: `* text=auto`, then `text eol=lf` for `*.md`, `*.toml`, `*.yaml`, `*.yml`,
  `*.json`, `*.py`, and also `*.code-snippets`, `.gitattributes` and `.gitignore`. This
  overrides `core.autocrlf=true` on Windows.
- `.gitignore`: `.theos/cache/`, `.worktrees/` (phase worktrees in this repo), `attachments/`
  (§4.6), Obsidian's per-user state (`.obsidian/workspace.json`, `.obsidian/workspace-mobile.json`,
  `.trash/`), Python caches (`__pycache__/`, `.venv/`, `.pytest_cache/`) and OS files
  (`.DS_Store`, `Thumbs.db`). Jann's untracked `.idea/` is left alone (not ignored, not added).

### 4.4 READMEs

**Root README** (CE-01.4) has these sections, under these headings so tests can find them:
- *Where each kind lives*: a table of all 20 kinds of data model §6 with their folder and id
  pattern (§3), plus how trees nest (children in a folder next to their parent; versions next to
  their concept or map).
- *System canon*: system canon (Shared Formalism and the other system pages) stays in
  Confluence; `canon/formalism/` is a generated read-only mirror; nothing else is copied.
  No Confluence URLs or page contents are written (this session cannot read Confluence).
- *Binaries*: binaries never enter git; they live in a separate binaries folder each user keeps
  in sync themselves and sets per machine in the editor's user settings; records point to them
  as `path@sha256:<hex>` (data model §10). Files pasted into Obsidian land in the ignored
  `attachments/` folder and are never committed.
- *Local disk*: the clone must be on a local disk, not a synced or streamed drive (OneDrive,
  Google Drive, Dropbox, iCloud), because git, the index cache and the repo lock break there.
- *Making a record*: the Obsidian and VS Code steps of §4.6 and §4.7.
- *Checks*: how to run the tests of §4.8.

The data model is linked by its GitHub URL in `theos-editor`.

**Folder READMEs** say what lives in the folder, the id pattern, how children nest, and which
template to use. `canon/README.md` says the mirror is generated by `theos` and hand edits fail
the check (CE-16.3). `tools/README.md` names what will live there (`theos_maps` from P08,
`comfy/workflows` from P16, import scripts, `repo_checks`).

### 4.5 Record templates (CE-03.1)

One file per kind: `templates/records/<kind>.template.md`, 20 files, one for each kind of data
model §6 (`tower.template.md` … `map-version.template.md`). Nothing else lives in
`templates/records/`. The `.template` part is required. Obsidian writes a bare `[[name]]` only
when exactly one file in the vault has that name, and its name lookup ignores excluded folders.
A template named `tower.md` would turn every inserted link to the tower record
(`world/tower.md`) into `[[world/tower]]`, and the same would happen to any future record whose
id is a kind name. Ids cannot contain a dot, so `<kind>.template` can never equal an id.

**How a record is made in Obsidian** (core Templates plugin, decided by Jann): create a new note
in the right folder, give it the id as its name, then run "Templates: Insert template" in the
empty note. Obsidian 1.14 replaces `{{title}}` with the note's file name (case-insensitive) and
`{{date}}` / `{{time}}` with the current time. A template inserted at the very start of an
empty note goes in byte for byte; inserted anywhere else, Obsidian merges its front matter
through its own YAML writer. The root README and every folder README say "empty note, then
insert".

**What "valid front matter" means** (used by the tests): the record, with `{{title}}` replaced
by a valid id of the kind,
1. parses under the restricted YAML of data model §4;
2. has `id` equal to the file name and matching the kind's pattern (§3), and the right `kind`;
3. has every field the kind requires (§5, §7, §8) with a value of the right type, pattern or
   allowed value;
4. has no field outside the kind's tables.

Whether the links resolve is not part of it: dangling links are findings of the index (CE-12.3),
and placeholders are meant to dangle until filled in.

**Template rules**
- Front matter holds `id: {{title}}`, `kind`, `title: {{title}}`, `status` where it is required,
  and every required field of the kind. Optional fields, including `tags`, are left out; the
  folder READMEs list them. No comments in front matter, so the records stay clean.
- Field order: the §5 fields first (`id`, `kind`, `title`, `status`), then the kind's fields in
  the order of its table. The examples for clue and script already use this order; the floor
  example has `status` last. Both are valid, and the templates follow the majority.
- No `{{date}}` or `{{time}}`: dates live in git, not in records (data model §2).
- The body is one short line in the voice the body needs, telling the author what to write.
  It is an instruction, never lore. The bodies of `equation`, `concept-version` and
  `map-version` say these records are normally written by the editor or the mirror; a
  hand-made equation fails the mirror check.
- The body of the `template` kind's template may show compile placeholders, but none named
  `title`, `date` or `time` (Obsidian would replace them).

**Placeholders** for values nobody can know when the record is made. Every placeholder is valid
for its field, and every link placeholder dangles, so the index reports it until it is filled:

| Value | Placeholder |
|---|---|
| text | `todo` |
| link to a slug kind | `todo` |
| link to truth, belief, clue, script | `T-000`, `B-000`, `C-000`, `S-000` |
| link to a floor | `f00-todo` |
| list of links | `[todo]` (one-line list) |
| URL | `https://todo.invalid/` |
| `generator` | `todo@0.0.0` |
| binary ref | `todo@sha256:` + 64 zeros |

**Defaults for required choices** fail safe: `truth.target: never` (locked, withheld from LLM
prompts, never revealed until the author lowers it), `clue.moves_to: hinted` (smallest step),
`status: draft`, `rating: unrated`. Everything else uses the first allowed value.

**Per kind** (after `id`, `kind`, `title`):

| Kind | Fields in the template |
|---|---|
| tower | – |
| band | `parent: tower`, `floors: [1, 1]` |
| floor | `status: draft`, `parent: todo`, `number: 1` |
| location | `parent: todo` |
| people | – |
| faction | `parent: todo` |
| npc | `parent: todo` |
| thing | `type: todo` |
| topic | `status: draft`, `track: explaining` |
| truth | `status: draft`, `target: never` |
| belief | `status: draft`, `held_by: [todo]` |
| clue | `status: draft`, `reveals: T-000`, `moves_to: hinted`, `where: [todo]`, `gate: T1`, `vector: todo` |
| equation | `status: canon`, `source: https://todo.invalid/`, `generated: true` |
| script | `status: draft` |
| dialogue | `speaker: todo` |
| template | `purpose: dialogue` |
| concept | `for: [todo]` |
| concept-version | `concept: todo`, `generator`, `recipe: todo`, `outputs` with one role `image`, `rating: unrated` |
| map | `for: [todo]` |
| map-version | `map: todo`, `generator`, `recipe: todo`, `outputs` with one role `image`, `rating: unrated` |

Notes: the band's parent is always the tower, whose id is `tower` (`world/tower.md`).
`equation` carries `status: canon`; that is valid whether or not the equation counts as a lore
kind, so P04 can decide. The topic template is the explaining one; an in-world topic is made from
it by setting `track: in-world` and adding `counterpart` (the VS Code snippet does both from the
file name, §4.7).

### 4.6 Obsidian settings (CE-03.2)

The vault is the repo root. Tracked files in `.obsidian/`, with keys checked against the
Obsidian 1.14.4 app bundle:

- `templates.json`: `{"folder": "templates/records"}`.
- `core-plugins.json`: the object form, with `templates: true`. It is written with the full
  default set Obsidian 1.14 saves, so that opening the vault changes nothing; missing keys would
  fall back to each plugin's default and make Obsidian rewrite the file.
- `app.json`:
  - `useMarkdownLinks: false` (wikilinks) and `newLinkFormat: "shortest"`, so inserted links are
    `[[id]]` and `[[id]]` resolves to the file named `id.md`. Ids are unique, so the shortest
    form is always the bare id;
  - `alwaysUpdateLinks: false`: Obsidian can rename body links but not `id:` or front-matter
    links, so a rename there would half-rename a record. Renames belong to the editor (CE-20.3);
  - `propertiesInDocument: "source"`: front matter is shown and edited as YAML text. The
    Properties widget re-serialises the whole block and can produce YAML the restricted parser
    rejects;
  - `attachmentFolderPath: "attachments"` (ignored, §4.3);
  - `newFileLocation: "current"`, so a new note lands next to the one being viewed;
  - `userIgnoreFilters`: `templates/records/`, `tools/`, `attachments/`, so templates and tool
    files do not show up in search, the graph or `[[` suggestions.

`.obsidian/` holds no community plugins, themes or workspace files.

### 4.7 VS Code snippets (CE-03.2)

One workspace snippet file, `.vscode/theos.code-snippets`, scope `markdown`, one snippet per kind
with prefix `theos-<kind>` (`theos-npc`, `theos-map-version`). Typed into an empty file and
accepted, it writes the same front matter and body line as the Obsidian template, but VS Code can
derive more:

- `id` from `$TM_FILENAME_BASE`;
- `parent` (floor, location, faction, npc), `concept` and `map` (versions) from the last segment
  of `$TM_DIRECTORY`, accepting both `/` and `\` as separators;
- the floor's `number` from its id (`f04-choir-hall` → `4`);
- the topic's `track` from the id suffix, and for `-in-world` ids a `counterpart` line pointing at
  the matching `-explaining` id;
- required choices as choice lists with the fail-safe default first (`${n|never,hinted,…|}`);
- other placeholders as tab stops whose default is the placeholder of §4.5.

Snippets use only this syntax subset, so the tests can expand them: `$TM_FILENAME_BASE`,
`$TM_DIRECTORY`, `${VAR/regex/format/}` with `$n` and `${n:+…}` in the format, `${n:default}`,
`${n|a,b|}`, `$0`. The regexes avoid constructs that behave differently in JavaScript and Python.
No `.vscode/settings.json` is required; adding `files.eol` and markdown suggestions is left to the
builder.

### 4.8 Tests

There is no Rust in `tower-of-theos`, and P00's `ac-trace` cannot see it. The tests are a small
Python project, `tools/repo_checks/`: `pyproject.toml` (Python 3.12, pytest as the only
dependency), `uv.lock` committed, run with `uv run --project tools/repo_checks pytest`. Python and
uv are the toolchain the map toolset needs anyway. Tests carry `# AC: CE-01.1` comments directly
above them (the `docs/gate.md` §4 convention in Python syntax), find the repo root with
`git rev-parse --show-toplevel`, and need no network.

What they prove:

| AC | Test idea |
|---|---|
| CE-01.1 | Every path of §4.1 exists; each top-level folder has a non-empty `README.md`; `theos.toml` parses (`tomllib`) and equals data model §13; `.theos/settings.toml` parses. |
| CE-01.2 | `git check-attr eol` reports `lf` for a sample path of every extension; no tracked text file contains CR. |
| CE-01.3 | `git check-ignore` ignores `.theos/cache/x` and does not ignore `.theos/settings.toml`. |
| CE-01.4 | The root README has the four headings of §4.4; the kind table names all 20 kinds with the folder of data model §6. |
| CE-03.1 | `templates/records/` holds exactly the 20 files `<kind>.template.md`; each, made into a record the way Obsidian does (§4.5), has valid front matter by the four rules; no `{{date}}` or `{{time}}`; no template's file name equals a valid id of any kind. |
| CE-03.2 | `.obsidian/` files have the values of §4.6; the snippet file has one snippet per kind; each snippet, expanded for a sample path (with `/` and `\` directories), gives valid front matter and the derived values of §4.7. |

The tests contain a small restricted-YAML checker and a table of the kinds' required and allowed
fields. Both are test code, not a second implementation for others to use. The checker has its own
negative tests (anchors, tags, `|`, flow maps, tabs, duplicate keys rejected), so a checker that
accepts everything cannot pass. When `theos check` exists (P05), the template tests can switch to
it.

`.github/workflows/checks.yml` runs the tests on `ubuntu-latest` and `windows-latest` for pull
requests and pushes to `main`, using `astral-sh/setup-uv` pinned to a release.

### 4.9 How the build runs

Branch `phase/P01-content-layout-and-templates` in a worktree at
`../tower-of-theos/.worktrees/P01-content-layout-and-templates`. Until this PR merges,
`.worktrees/` is not yet ignored on `tower-of-theos` `main`, so the worktree is excluded through
`.git/info/exclude` in the clone (local only, not committed). Commits follow theos-editor's
`CONTRIBUTING.md` §2 (scope `content`, `Refs:` and `Phase: P01` footers, Claude trailers). The PR
is `feat(content): content layout and templates (P01)` with theos-editor's template sections. It
waits for Jann, since `tower-of-theos` has no auto-merge.

The plan and the phase state stay in this repo. The plan is `docs/plans/P01-content-layout-and-templates.md`,
and the phase state is in `docs/status.md`. Both go on a companion branch
`phase/P01-content-layout-and-templates` in a worktree `.worktrees/P01-content-layout-and-templates`.
Its PR, `docs(docs): plan and status of content layout and templates (P01)`, touches no protected
path. Nothing from the plan is copied into `tower-of-theos`.

## 5. Interfaces other phases use

- **Not records** (P03 index, P04 checks, P05 `theos check`): every `README.md`, every file under
  `templates/records/`, and everything under `.theos/`, `.obsidian/`, `.vscode/`, `.github/`,
  `tools/`, `attachments/` and `.worktrees/`. The index skips them; it does not report them as
  records with wrong ids or as files outside their kind folder.
- **Placeholders** (§4.5 table) are reserved: no real record uses `todo`, `T-000`, `B-000`,
  `C-000`, `S-000` or `f00-todo` as its id. P04 may add a warning for front matter that still
  holds a placeholder; P01 does not require one.
- **Field order** of the templates (§4.5) is the order P02's patcher uses to insert new fields.
- **Templates are hand-kept** until `theos new` (P05) exists; P05 may generate them from the kind
  registry, and its output must stay valid by the same four rules. Template file names keep the
  form `<kind>.template.md`, which can never equal an id.
- **YAML-special ids** (P02, P03): a slug id that a plain scalar reads as non-text (`true`,
  `false`, digits only) matches the id pattern of data model §3 but would not be text. The
  templates keep `id` and `title` unquoted, as the data model's examples do. P02 and P03 decide
  how such an id is handled; the open question is listed under "Needs Jann" in `docs/status.md`.
- **P00 `ac-trace`**: the ACs of P01 (and P08) are proven by tests in `tower-of-theos`, tagged
  `# AC: <id>`. `ac-trace` in this repo must not demand tests for them here.
- **P14 "new content project"** (CE-38.2) creates the same tree; this layout is the reference.
- **P08 and P16** add `tools/theos_maps/` and files under `tools/comfy/workflows/`.

## 6. Decisions

Decided by Jann (31 Oct 2026):
- Obsidian uses the core Templates plugin; no community plugins.
- Accepted Claude's proposals: valid front matter as in §4.5 with dangling placeholders;
  templates for all 20 kinds, generated kinds included, with a note that they are normally
  generated; Obsidian links in the short `[[id]]` form, no link rewrite on rename, front matter
  shown as YAML source, pasted files in an ignored folder; Python tests with a CI workflow on
  Windows and Linux; `README.md` files and `templates/records/` are not records; no content
  records; `.worktrees/` ignored.

Claude's calls inside that frame (Jann checks them in this review):
- placeholder values and fail-safe defaults (§4.5);
- no comments in template front matter; optional fields only in READMEs;
- field order with `status` after `title`;
- one topic template (explaining); in-world by editing, or derived by the snippet;
- template files named `<kind>.template.md` so no template name can equal an id (§4.5);
- `id` and `title` unquoted in templates, as in the data model; YAML-special ids left to P02/P03;
- the plan and status on a companion branch in this repo (§4.9);
- `equation` template with `status: canon`;
- the generated-kind note in the template body rather than in a front-matter comment;
- `attachments/` as the ignored paste folder; `newFileLocation` and `userIgnoreFilters` (§4.6);
- the other not-a-record folders of §5;
- `.gitkeep` in empty leaf folders; READMEs only in top-level folders;
- `.theos/settings.toml` starts as a comment only.

## 7. Risks

- **Obsidian rewrites its settings on first open.** It may add keys to `app.json` or
  `core-plugins.json` that the builder cannot predict without running Obsidian. Mitigation: write
  the files as 1.14.4 writes them. Jann's check before merging is to open the vault once and see
  that `git status` stays clean; any diff is folded into the PR.
- **Template inserted into a note that is not empty, or before the note is named.** Obsidian
  merges front matter through its own writer, or `{{title}}` becomes `Untitled`. Mitigation: the
  READMEs say "empty note, then insert"; `Untitled` fails the id pattern, so P04 reports it.
- **Placeholders that look real.** `number: 1`, `floors: [1, 1]` and `gate: T1` cannot be told
  apart from deliberate values. Accepted: no placeholder in those fields is both valid and
  detectable. The guardrail-relevant choices fail safe.
- **The test checker drifts from P02's real parser.** Mitigation: it covers only what templates
  use and is replaced by `theos check` once P05 exists.
- **The VS Code regex dialect.** JavaScript and Python regexes differ at the edges. Mitigation:
  the subset rule of §4.7, plus tests with both separators.
- **CI minutes** on a private repo, with Windows billed double. The job takes seconds; accepted.

## 8. Review Focus

Inputs most likely to break this phase:
- Windows checkouts with `core.autocrlf=true`, and files created on Windows by Obsidian or VS Code;
- `$TM_DIRECTORY` with backslashes, drive letters, or a file directly in a kind folder (no parent
  folder to derive from);
- `{{Title}}` in other casing, and `$` or other special characters in a replacement;
- ids that look YAML-special but are text under data model §4 (`no`, `yes`, `null`, `E7`,
  `T-012`): these must stay text in the checker. `true`, `false` and digit-only slugs are the
  open question of §5 and are not used as sample ids;
- a vault where some record's id equals a kind name (`world/tower.md`): inserted links stay
  bare `[[tower]]`;
- `f00-…` and `f100-…` floor ids in the number derivation;
- `-in-world` topic ids for the counterpart derivation;
- a template inserted into a note that already has front matter;
- `git check-ignore` with and without a trailing slash, and `.theos/cache` nested paths.

## 9. Left to the builder

- Exact README wording, body lines of the templates, and snippet descriptions.
- Exact Obsidian default set in `core-plugins.json` (taken from 1.14.4).
- Test file layout inside `tools/repo_checks/`, and the sample ids used per kind.
- The pinned versions of pytest, uv and `astral-sh/setup-uv`.
- Optional `.vscode/settings.json` and `.vscode/extensions.json`.
