# P01 · Content layout and templates — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `tower-of-theos` the content layout, `theos.toml`, line-ending and ignore rules,
READMEs, one record template per kind, Obsidian settings and VS Code snippets of the P01 spec,
with tests that prove CE-01 and CE-03 on Windows and Linux.

**Architecture:** Plain files in the content repo, plus one small Python test project
(`tools/repo_checks`). It holds a restricted-YAML checker, a kind table, an Obsidian template
renderer and a VS Code snippet expander, all test code that emulates what the real tools do. The
tests read the committed files and the git index; nothing generates the content files.

**Tech Stack:** Python 3.12, uv, pytest, git CLI, GitHub Actions (`ubuntu-latest`,
`windows-latest`).

**Spec:** `docs/specs/phases/P01-content-layout-and-templates.md` (theos-editor). Read it before
any task; this plan refers to its sections as "spec §N".

**Where the work happens**
- Build: `D:\Projects\tower-of-theos\.worktrees\P01-content-layout-and-templates`, branch
  `phase/P01-content-layout-and-templates` (called `<repo>` below). Every path below is relative to `<repo>`.
- This plan and `docs/status.md` live in theos-editor, branch
  `phase/P01-content-layout-and-templates`, worktree `.worktrees/P01-content-layout-and-templates`.
- Test command (from `<repo>`): `uv run --project tools/repo_checks pytest tools/repo_checks -q`.
  The path argument makes pytest use `tools/repo_checks/pyproject.toml` as its config and keeps
  later Python projects under `tools/` out of the run. This extends the bare command written in spec §4.8.

## Global Constraints

Umbrella (`docs/specs/00-umbrella.md` §2), verbatim:

- **G1** Repositories: `theos-editor` (this repo) holds the editor. `tower-of-theos` holds the
  content project and the map toolset. `Theos-Engine` is never a dependency, except behind the
  cargo feature `engine`, which no phase here builds.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Windows and Linux; macOS is tested weekly
  and before every release (`docs/gate.md` §1).
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

Phase ACs (umbrella §3 P01), verbatim:

- CE-01.1 `theos.toml`, `.theos/` and the folder tree of `docs/architecture.md` §2 exist, each
  top-level folder with a README.
- CE-01.2 `.gitattributes` forces LF for `md`, `toml`, `yaml`, `json`, `py`.
- CE-01.3 `.theos/cache` is ignored.
- CE-01.4 The root README says where each kind lives, that system canon stays in Confluence,
  that binaries live in a separate folder the user syncs, and that the clone must be on a local disk.
- CE-03.1 One template per kind in `templates/records/`; a record made from it in Obsidian has
  valid front matter.
- CE-03.2 Obsidian settings in `.obsidian/` point at the templates and treat `[[id]]` links as
  file links; VS Code snippets exist for every kind.

Phase-specific (spec), one line each:

- Commits follow theos-editor `CONTRIBUTING.md` §2: Conventional Commits, scope `content` (`ci` for
  the workflow), footers `Refs: <AC ids>`, `Phase: P01`,
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`,
  `Claude-Session: a4d54aa3-4608-4b64-add7-29193c5cbc03` (G17).
- Every text file written is UTF-8 with LF, including on Windows.
- No lore: sample ids come from data model §14 or are named `sample-<kind>`; nothing in a README
  or template invents canon. No content records anywhere outside the test code.
- A test proves an AC with a comment `# AC: <id>` on the line directly above the test function.
- Tests need no network once `uv sync` has run, and run unchanged on Windows and Linux.
- Never touch the `tower-of-theos` main checkout. `.worktrees/` is already listed in its
  `.git/info/exclude`.

## Review Focus

Inputs the spec implies, beyond what the ACs name, most likely to bite first:

1. **A Windows checkout with `core.autocrlf=true`**: files of the forced extensions are LF in the
   working tree too. Test: Task 1 `test_forced_extensions_are_lf_in_index_and_worktree`.
2. **`$TM_DIRECTORY` with backslashes and a drive letter, or a file directly in a kind folder**:
   the parent comes out right, or `todo`. Test: Task 7 `test_parent_is_todo_directly_in_kind_folder`
   and the `\` cases of `test_snippet_derives_values`.
3. **Ids that look YAML-special but are text** (`no`, `yes`, `null`, `E7`, `T-012`, `f04-x`): they
   stay strings. Test: Task 3 `test_text_looking_like_other_types_stays_text`.
4. **Floor ids `f00-…`, `f100-…` and `-in-world` topic ids** in the snippet derivations. Test:
   Task 7 `test_floor_number_edges` and `test_in_world_topic_gets_counterpart`.
5. **`.theos/cache` without a trailing slash, and nested paths below it**: ignored either way. Test:
   Task 1 `test_theos_cache_is_ignored`.

---

## File structure (`<repo>`)

| Path | Responsibility | Task |
|---|---|---|
| `.gitattributes`, `.gitignore` | LF and ignore rules (spec §4.3) | 1 |
| `.github/workflows/checks.yml` | Runs the tests on Ubuntu and Windows | 1 |
| `tools/repo_checks/pyproject.toml`, `.python-version`, `uv.lock` | Test project | 1 |
| `tools/repo_checks/repo_checks/repo.py` | Repo root and git helpers | 1 |
| `theos.toml`, `.theos/README.md`, `.theos/settings.toml` | Manifest and project settings (spec §4.2) | 2 |
| `<top-level>/README.md`, `.gitkeep` in empty leaves | Folder tree (spec §4.1, §4.4) | 2 |
| `tools/repo_checks/repo_checks/ryaml.py` | Restricted-YAML checker | 3 |
| `tools/repo_checks/repo_checks/kinds.py` | Kind table and front-matter validator | 4 |
| `tools/repo_checks/repo_checks/samples.py` | Sample ids and paths per kind | 4 |
| `tools/repo_checks/repo_checks/layout.py` | Top-level folder list | 2 |
| `tools/repo_checks/repo_checks/obsidian.py`, `expected.py` | Obsidian template renderer; expected template fields | 5 |
| `templates/records/<kind>.template.md` ×20 | Record templates (spec §4.5) | 5 |
| `.obsidian/app.json`, `core-plugins.json`, `templates.json` | Obsidian settings (spec §4.6) | 6 |
| `tools/repo_checks/repo_checks/snippets.py` | VS Code snippet expander | 7 |
| `.vscode/theos.code-snippets` | Snippets (spec §4.7) | 7 |
| `README.md` | Root README (spec §4.4) | 8 |

Tests live in `tools/repo_checks/tests/test_<topic>.py`, one file per task.

---

### Task 1: Test harness, line endings, ignore rules, CI

**ACs:** CE-01.2, CE-01.3

**Files:**
- Create: `tools/repo_checks/pyproject.toml`, `tools/repo_checks/.python-version` (`3.12`),
  `tools/repo_checks/uv.lock`, `tools/repo_checks/repo_checks/__init__.py`,
  `tools/repo_checks/repo_checks/repo.py`, `.gitattributes`, `.gitignore`,
  `.github/workflows/checks.yml`
- Test: `tools/repo_checks/tests/test_git_rules.py`

**Interfaces:**
- Produces: `repo_root() -> pathlib.Path` (runs `git rev-parse --show-toplevel` from the package
  directory and caches the result); `git(*args: str) -> str` (runs `git` in `repo_root()`, returns
  stdout, raises `subprocess.CalledProcessError` on a non-zero exit); `git_ok(*args: str) -> bool`
  (exit code 0); `read_text(rel: str) -> str` (reads `repo_root() / rel` as UTF-8 with
  `newline=""`, so a CR stays visible to tests).

- [ ] **Step 1: Scaffold the test project.** `pyproject.toml`: `name = "repo-checks"`,
  `version = "0.1.0"`, `requires-python = ">=3.12,<3.13"`, no `[build-system]` (uv treats it as a
  virtual project), `dependencies = []`, `[dependency-groups] dev = ["pytest>=8"]`,
  `[tool.pytest.ini_options] testpaths = ["tests"]`, `pythonpath = ["."]`. Run
  `uv lock --project tools/repo_checks` to write `uv.lock`.

- [ ] **Step 2: Write the failing tests** in `test_git_rules.py`:

```python
FORCED = ["md", "toml", "yaml", "yml", "json", "py", "code-snippets"]

# AC: CE-01.2
def test_gitattributes_forces_lf_for_required_extensions():
    for ext in FORCED:
        assert git("check-attr", "eol", "--", f"sample.{ext}").strip() == f"sample.{ext}: eol: lf"
        assert git("check-attr", "text", "--", f"sample.{ext}").strip() == f"sample.{ext}: text: set"

# AC: CE-01.2
def test_forced_extensions_are_lf_in_index_and_worktree():
    # `git ls-files --eol`: "i/<eol> w/<eol> attr/<attrs>\t<path>"
    # every tracked file whose extension is in FORCED: i/lf (or i/none if empty) and w/lf (or w/none)
    # every other tracked text file: index is not i/crlf and not i/mixed

# AC: CE-01.3
def test_theos_cache_is_ignored():
    for path in [".theos/cache", ".theos/cache/index.json", ".theos/cache/worktrees/s1/a.md"]:
        assert git_ok("check-ignore", "-q", "--no-index", path), path
    for path in [".theos/settings.toml", ".theos/README.md", ".theos/sessions/x.json"]:
        assert not git_ok("check-ignore", "-q", "--no-index", path), path

def test_local_state_is_ignored_and_obsidian_settings_are_not():
    # ignored: .worktrees/x, attachments/a.png, .obsidian/workspace.json, .obsidian/graph.json,
    #          .obsidian/plugins/x/main.js, .trash/a.md, tools/repo_checks/.venv/x,
    #          a/__pycache__/m.pyc, .pytest_cache/x, .DS_Store, Thumbs.db
    # not ignored: .obsidian/app.json, .obsidian/core-plugins.json, .obsidian/templates.json,
    #          tools/repo_checks/uv.lock
```

- [ ] **Step 3: Run the tests to verify they fail.** Run the test command. Expected: FAIL; there
  are no `.gitattributes` or `.gitignore` yet.

- [ ] **Step 4: Write `.gitattributes`.** Content: `* text=auto`, then one `text eol=lf` line each
  for `*.md *.toml *.yaml *.yml *.json *.py *.code-snippets .gitattributes .gitignore`
  (a separate line per pattern), and `*.lock text eol=lf` so `uv.lock` matches CI on Windows.

- [ ] **Step 5: Write `.gitignore`.** Anchored patterns without trailing slashes, so a path is
  ignored whether or not it exists as a folder: `/.theos/cache`, `/.worktrees`, `/attachments`,
  `/.trash`. Obsidian allowlist:
  `/.obsidian/*`, `!/.obsidian/app.json`, `!/.obsidian/core-plugins.json`,
  `!/.obsidian/templates.json`. Then `__pycache__/`, `.venv/`, `.pytest_cache/`, `.DS_Store`,
  `Thumbs.db`. A comment line above each group says why. `.idea/` is not listed.
  (Ruling: the allowlist is a stronger form of spec §4.3's per-file list. Obsidian creates other
  per-user files (`appearance.json`, `graph.json`, `types.json`) on first open, so this keeps
  `git status` clean.)

- [ ] **Step 6: Implement `repo.py`** with the three functions of the Interfaces block.

- [ ] **Step 7: Stage everything and run the tests.** `git add -A`, then the test command.
  Expected: PASS, 4 tests. (The `ls-files` test sees only staged files.)

- [ ] **Step 8: Write `.github/workflows/checks.yml`.** Name `checks`, triggers `pull_request`
  and `push` to `main`, job `repo-checks` with matrix `os: [ubuntu-latest, windows-latest]`,
  steps: `actions/checkout` and `astral-sh/setup-uv`, each pinned to a current release tag, then
  `uv run --project tools/repo_checks --locked pytest tools/repo_checks -q`. `permissions:
  contents: read`. No caching or other steps.

- [ ] **Step 9: Commit** — `feat(content): add repo checks, LF and ignore rules` with
  `Refs: CE-01.2, CE-01.3`, plus the footers of the Global Constraints.

---

### Task 2: Manifest, project settings and the folder tree

**ACs:** CE-01.1

**Files:**
- Create: `tools/repo_checks/repo_checks/layout.py`, `theos.toml`, `.theos/README.md`,
  `.theos/settings.toml`; `README.md` in `world`,
  `people`, `things`, `lore`, `story`, `concepts`, `maps`, `templates`, `dialogue`, `canon`,
  `tools`; empty `.gitkeep` in `lore/topics`, `lore/truths`, `lore/beliefs`, `lore/clues`,
  `story/scripts`, `canon/formalism`, `tools/comfy/workflows`
- Test: `tools/repo_checks/tests/test_layout.py`

**Interfaces:**
- Consumes: `repo_root()` (Task 1).
- Produces: `TOP_LEVEL_FOLDERS = (".theos", "world", "people", "things", "lore", "story",
  "concepts", "maps", "templates", "dialogue", "canon", "tools")` in
  `repo_checks/layout.py`, used by Task 5's README test.

- [ ] **Step 1: Write the failing tests:**

```python
# AC: CE-01.1
def test_theos_toml_is_the_data_model_manifest():
    data = tomllib.loads((repo_root() / "theos.toml").read_text(encoding="utf-8"))
    assert data == {"project": {"name": "tower-of-theos", "kind": "content", "schema_version": 1},
                    "parts": {"enabled": ["codex", "changes", "scripts", "graph", "concepts",
                                          "maps", "review", "mcp"]}}

# AC: CE-01.1
def test_theos_settings_is_tracked_toml():
    # .theos/settings.toml parses with tomllib to {} (comments only) and is in `git ls-files`

# AC: CE-01.1
def test_every_top_level_folder_has_a_readme():
    for folder in TOP_LEVEL_FOLDERS:
        readme = repo_root() / folder / "README.md"
        assert readme.is_file() and readme.read_text(encoding="utf-8").strip(), folder

# AC: CE-01.1
def test_folder_tree_of_the_architecture_exists():
    for leaf in ["lore/topics", "lore/truths", "lore/beliefs", "lore/clues", "story/scripts",
                 "canon/formalism", "tools/comfy/workflows"]:
        assert (repo_root() / leaf / ".gitkeep").is_file(), leaf
```

`templates/records/` is proven by Task 5's `test_one_template_per_kind`.

- [ ] **Step 2: Run the tests to verify they fail.** Expected: FAIL; the files are missing.

- [ ] **Step 3: Write the files.**
  - `theos.toml`: data model §13, byte for byte.
  - `.theos/settings.toml`: two comment lines (spec §4.2).
  - `.theos/README.md`: spec §4.2.
  - Folder READMEs per spec §4.4, each with:
    - a one-line purpose;
    - for each kind in the folder: its location (data model §6), its id pattern (§3), how
      children nest (§2), its template file `<kind>.template.md`, and its optional fields (data
      model §7–§8, every field the template leaves out).

    Extra lines: `canon/README.md` says the mirror is generated and hand edits fail the check.
    `tools/README.md` lists `theos_maps` (P08), `comfy/workflows` (P16), import scripts and
    `repo_checks`. `templates/README.md` explains `records/` vs compile templates of kind
    `template`. Each README that has kinds ends with the Obsidian steps: "create an empty note
    named with the id, then run *Templates: Insert template*". Write the folder table once in
    `layout.py`.

- [ ] **Step 4: Run the tests to verify they pass** (`git add -A` first). Expected: PASS.

- [ ] **Step 5: Commit** — `feat(content): add theos.toml and the content folder tree`,
  `Refs: CE-01.1`.

---

### Task 3: Restricted-YAML checker

**ACs:** none directly (supports CE-03.1). Untagged tests.

**Files:**
- Create: `tools/repo_checks/repo_checks/ryaml.py`
- Test: `tools/repo_checks/tests/test_ryaml.py`

**Interfaces:**
- Produces:
  - `Scalar = str | int | Decimal | bool`; `Value = Scalar | list[Scalar] | dict[str, Scalar | list[Scalar]]`
  - `class RestrictedYamlError(ValueError)` with `line: int`, `column: int` (1-based, counted from
    the first line of the file) and `message: str`; `str(e) == f"{line}:{column}: {message}"`.
  - `@dataclass(frozen=True) class Record: fields: dict[str, Value]; body: str`
  - `parse_record(text: str) -> Record`: the text must start with `---\n`; front matter runs to
    the next line that is exactly `---`; `body` is everything after that line's `\n`. A missing
    delimiter raises `RestrictedYamlError`.

Scope is data model §4 and nothing more. Typing of plain scalars: exactly `true`/`false` → `bool`;
`-?[0-9]+` → `int`; `-?[0-9]+\.[0-9]+` → `Decimal`; anything else → `str` (so `no`, `yes`, `null`,
`~`, `E7`, `1e5` are strings). Single-quoted strings support `''`; double-quoted support `\"` and
`\\`. A comment starts at `#` that begins the line (after indentation) or follows whitespace
outside quotes. One-line lists `[a, b]` split on commas outside quotes; `[]` is an empty list.
Block lists: `key:` then `- item` lines at any indentation ≥ the key's. Maps: `key:` then
indented `sub: value` lines with one consistent indentation; values are scalars or one-line
lists.

- [ ] **Step 1: Write the failing tests.** Positive:

```python
def test_parses_data_model_npc_with_lens_map():  # data model §14 mother-ilse, verbatim
    r = parse_record(MOTHER_ILSE)
    assert r.fields["lens"] == {"speaks_to": 'children and newcomers; calls the player "stair-child"'}
    assert list(r.fields) == ["id", "kind", "title", "parent", "home", "role", "lens"]

def test_parses_lists_maps_numbers_and_comments():
    # "floors: [1, 20]" -> [1, 20]; "where:\n  - a\n  - b" -> ["a", "b"];
    # "results:\n  relief_m: 1234\n  lakes_pct: 2.99\n  pass: true" -> {..., Decimal("2.99"), True};
    # "title: A # note" -> "A"; "title: C#sharp" -> "C#sharp"; own-line and blank lines skipped

def test_text_looking_like_other_types_stays_text():
    for raw in ["no", "yes", "null", "~", "E7", "T-012", "f04-x", "1e5", "0x1F", "'true'", '"42"']:
        value = parse_record(f"---\nid: {raw}\n---\n").fields["id"]
        assert isinstance(value, str)
```

Negative: one test per rule, each asserting `(e.line, e.column)` and a word in the message.
- `&a` anchor, `*a` alias, `!tag`: line 2, column 7 for `title: &a x`;
- `|` and `>-` block scalars: "multi-line";
- `{a: 1}`: "flow map";
- `- a: 1` under a key: "list of maps";
- a map inside a map: "nested";
- a tab in indentation: "tab";
- a duplicate key: "duplicate", at the second key;
- `Title:` or `1x:`: "key";
- `parent:` with nothing after it and no block: "empty";
- an indented continuation line after a plain scalar: "multi-line";
- a missing closing `---`: "front matter".

- [ ] **Step 2: Run to verify they fail** (`ModuleNotFoundError`).

- [ ] **Step 3: Implement `ryaml.py`.** A line-by-line scanner; no third-party YAML library.

- [ ] **Step 4: Run to verify they pass.**

- [ ] **Step 5: Commit** — `test(content): add a restricted YAML checker for repo checks`,
  `Refs: CE-03.1`.

---

### Task 4: Kind table, front-matter validator, samples

**ACs:** none directly (supports CE-03.1, CE-03.2, CE-01.4). Untagged tests.

**Files:**
- Create: `tools/repo_checks/repo_checks/kinds.py`, `tools/repo_checks/repo_checks/samples.py`
- Test: `tools/repo_checks/tests/test_kinds.py`

**Interfaces:**
- Consumes: `Value` (Task 3).
- Produces:
  - `KIND_NAMES: tuple[str, ...]`: the 20 kinds in data model §6 order (`"tower"` … `"map-version"`).
  - `KIND_FOLDERS: dict[str, str]`: the §6 folder column, verbatim and without backticks
    (`"world/tower.md"`, `"world/"`, `"world/<band>/"`, `"under its floor or location"`, …,
    `"maps/<map>/"`).
  - `KIND_TOP_FOLDER: dict[str, str]`: kind → top-level folder (`location → "world"`,
    `equation → "canon"`, `template → "templates"`, …).
  - `ID_PATTERNS: dict[str, re.Pattern[str]]`, per data model §3:
    - slug kinds: `^[a-z0-9]+(-[a-z0-9]+)*$`;
    - truth, belief, clue, script: `^T-[0-9]{3,}$` and the B-, C-, S- equivalents;
    - equation: `^E[0-9]+$`;
    - floor: slug and `^f[0-9]{2}`;
    - topic: slug ending `-(explaining|in-world)`;
    - versions: slug ending `-v[0-9]+`.
  - `validate_front_matter(fields: Mapping[str, Value], file_stem: str) -> list[str]`: the four
    rules of spec §4.5. It returns readable problems, and `[]` means valid. Rule 2 for versions
    checks the shape only.
  - `version_parent_problems(fields: Mapping[str, Value], file_stem: str) -> list[str]`: for
    versions, the id must equal `<concept|map field>-v<n>`; `[]` for every other kind.
  - In `samples.py`: `@dataclass(frozen=True) class Sample: kind: str; id: str; path: str`
    (POSix-relative path of the record), and `SAMPLES: dict[str, Sample]`, one per kind:
    - `tower` `world/tower.md`
    - `hearth` `world/hearth.md`
    - `f04-choir-hall` `world/hearth/f04-choir-hall.md`
    - `fates-shrine` `world/hearth/f04-choir-hall/fates-shrine.md`
    - `hearth-folk` `people/hearth-folk.md`
    - `fates-cult` `people/hearth-folk/fates-cult.md`
    - `mother-ilse` `people/hearth-folk/fates-cult/mother-ilse.md`
    - `sample-thing` `things/sample-thing.md`
    - `spirit-magic-explaining` `lore/topics/…`
    - `T-012`, `B-016`, `C-031` in their `lore/` folders
    - `E7` `canon/formalism/E7.md`
    - `S-004` `story/scripts/S-004.md`
    - `sample-dialogue` `dialogue/…`
    - `sample-template` `templates/…`
    - `fates-shrine` `concepts/fates-shrine.md`
    - `fates-shrine-v5` `concepts/fates-shrine/fates-shrine-v5.md`
    - `f04-terrain` `maps/f04-terrain.md`
    - `f04-terrain-v1` `maps/f04-terrain/f04-terrain-v1.md`

    Also `IN_WORLD_TOPIC = Sample("topic", "spirit-magic-in-world", "lore/topics/spirit-magic-in-world.md")`.
    A module docstring marks these as synthetic test inputs taken from the data model's
    illustrations.

Field rules, from data model §5, §7, §8:
- **Required/allowed per kind:** exactly the tables. `status` is required for topic, truth,
  belief, clue, floor and script, and allowed on every kind. `counterpart` is required when the
  id ends in `-in-world`, and then `track` must be `in-world`; `track` must match the suffix.
- **Types:**
  - text = non-empty `str`; enum = one of the listed values (`accuracy: false` arrives as the
    bool `False` and is accepted);
  - link = `str` matching `ID_PATTERNS` of any allowed target kind; list of links = `list` of those;
  - floor `number` = `int` 1–100; band `floors` = list of 2 ints, 1 ≤ first ≤ last ≤ 100;
  - URL = `^https?://\S+$`; generator = `^[^@\s]+@[^@\s]+$`;
  - binary ref = `^[^@]+@sha256:[0-9a-f]{64}$`; `outputs` = non-empty map of role → binary ref;
  - `inputs`, `results` = maps; `addresses` = list of `^n[0-9]+$`; `seed` = `int`;
    `generated` = `True`;
  - `lens` = map with only the six lens fields of §7, typed as listed there.

- [ ] **Step 1: Write the failing tests.**
  - `test_data_model_examples_are_valid`: each §14 example (floor, npc, clue, script, map,
    map-version, concept-version) as a string. Where the example has a `…` hash, use 64 hex
    digits and add a comment saying so. Assert `validate_front_matter(...) == []` and
    `version_parent_problems(...) == []`.
  - `test_samples_match_their_patterns`: every `SAMPLES[k].id` matches `ID_PATTERNS[k]`, and its
    file stem equals its id.
  - Negative tests, each asserting the problem text names the field:
    - a missing required field;
    - an unknown field;
    - id ≠ file stem;
    - `T-12`;
    - `reveals: B-001`;
    - `target: maybe`;
    - floor `number: 0` and `101`;
    - `floors: [20, 1]` and `[1, 2, 3]`;
    - an in-world topic without `counterpart`;
    - `generator: comfyui`;
    - a 63-digit hash;
    - `generated: false`;
    - a lens with an unknown field;
    - `concept: other` against id `fates-shrine-v5` (`version_parent_problems` only).

- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement `kinds.py` and `samples.py`.** The tables are data; one generic
  checker walks them.
- [ ] **Step 4: Run to verify they pass.**
- [ ] **Step 5: Commit** — `test(content): add the kind table and front-matter validator`,
  `Refs: CE-03.1`.

---

### Task 5: Record templates

**ACs:** CE-03.1

**Files:**
- Create: `tools/repo_checks/repo_checks/obsidian.py`, `tools/repo_checks/repo_checks/expected.py`;
  `templates/records/<kind>.template.md` for the 20 kinds
- Test: `tools/repo_checks/tests/test_templates.py`

**Interfaces:**
- Consumes: `read_text` (Task 1); `parse_record` (Task 3); `KIND_NAMES`, `KIND_TOP_FOLDER`,
  `ID_PATTERNS`, `validate_front_matter`, `SAMPLES` (Task 4).
- Produces:
  - `template_text(kind: str) -> str` in `obsidian.py`: `read_text(f"templates/records/{kind}.template.md")`.
  - `render_template(template: str, note_name: str) -> str`: emulates Obsidian 1.14.4's
    insertion into an empty note. It replaces every `{{title}}`, case-insensitively, with
    `note_name` taken literally, and every `{{date}}`, `{{date:<fmt>}}`, `{{time}}` and
    `{{time:<fmt>}}` with the fixed text `2026-10-31` / `12:00`. Other `{{…}}` stay.
  - `uses_date_or_time(template: str) -> bool`.
  - `EXPECTED_FIELDS: dict[str, dict[str, Value]]` in `expected.py`: per kind, the exact front
    matter after `id`, `kind` and `title`, in order, from spec §4.5 "Per kind". Task 7 reuses it.

Template format (each file ends with one `\n`; no comments; no blank lines inside front matter):

```text
---
id: {{title}}
kind: npc
title: {{title}}
parent: todo
---
Who this person is, what they want and how they speak, in a few lines.
```

Field values are exactly spec §4.5 "Per kind" and "Placeholders". Binary ref placeholder:
`todo@sha256:` + 64 `0`. Body: one line telling the author what to write, never lore. The bodies
of `equation`, `concept-version` and `map-version` contain the words "normally written by". The
`template` body shows at least one compile placeholder (for example `{{speaker}}`), and none
named `title`, `date` or `time`.

- [ ] **Step 1: Write the failing tests:**

```python
# AC: CE-03.1
def test_one_template_per_kind():
    names = sorted(p.name for p in (repo_root() / "templates" / "records").iterdir())
    assert names == sorted(f"{k}.template.md" for k in KIND_NAMES)

# AC: CE-03.1
@pytest.mark.parametrize("kind", KIND_NAMES)
def test_record_made_in_obsidian_has_valid_front_matter(kind):
    text = template_text(kind)
    sample = SAMPLES[kind]
    record = parse_record(render_template(text, sample.id))
    assert validate_front_matter(record.fields, sample.id) == []
    assert record.fields == {"id": sample.id, "kind": kind, "title": sample.id, **EXPECTED_FIELDS[kind]}
    assert list(record.fields) == ["id", "kind", "title", *EXPECTED_FIELDS[kind]]

# AC: CE-03.1
def test_templates_hold_no_date_time_comments_or_crlf():
    # for each template: not uses_date_or_time; no line in front matter starts with "#";
    # no "\r"; ends with exactly one "\n"

# AC: CE-03.1, CE-03.2
def test_no_template_name_can_be_an_id():
    for k in KIND_NAMES:
        stem = f"{k}.template"
        assert not any(p.match(stem) for p in ID_PATTERNS.values())

def test_generated_kinds_say_so_and_compile_placeholders_survive():
    # bodies of equation, concept-version, map-version contain "normally written by";
    # render_template of the template kind's template still contains "{{" in the body

def test_render_template_matches_obsidian():
    assert render_template("a {{Title}} {{TITLE}} {{title}}", "T-012") == "a T-012 T-012 T-012"
    assert render_template("{{date}} {{time:HH}} {{speaker}}", "x") == "2026-10-31 12:00 {{speaker}}"

def test_folder_readmes_name_their_templates():
    for k in KIND_NAMES:
        assert f"{k}.template.md" in read_text(f"{KIND_TOP_FOLDER[k]}/README.md"), k
```

- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement `obsidian.py` and `expected.py`, and write the 20 templates.**
- [ ] **Step 4: Run all tests (`git add -A` first).** Expected: PASS.
- [ ] **Step 5: Commit** — `feat(content): add one record template per kind`, `Refs: CE-03.1`.

---

### Task 6: Obsidian settings

**ACs:** CE-03.2

**Files:**
- Create: `.obsidian/app.json`, `.obsidian/core-plugins.json`, `.obsidian/templates.json`
- Test: `tools/repo_checks/tests/test_obsidian.py`

**Interfaces:**
- Consumes: `repo_root()`, `git()` (Task 1).

All three files are written exactly as Obsidian writes them, `JSON.stringify(value, undefined, 2)`:
2-space indent and **no trailing newline**. The test compares each file's text with
`json.dumps(json.loads(text), indent=2)`.

`core-plugins.json`, in this order:
- `true`: `file-explorer`, `global-search`, `switcher`, `graph`, `backlink`, `outgoing-link`,
  `tag-pane`, `properties`, `page-preview`, `daily-notes`, `templates`, `note-composer`,
  `command-palette`, `editor-status`, `bookmarks`, `outline`, `word-count`, `canvas`, `bases`,
  `file-recovery`, `sync`;
- `false`: `slash-command`, `footnotes`, `webviewer`, `markdown-importer`, `zk-prefixer`,
  `random-note`, `slides`, `audio-recorder`, `workspaces`, `publish`, `starred`.

These are the core plugins of Obsidian 1.14.4 with their default state, read from its app
bundle. An object that is missing a registered plugin makes Obsidian rewrite the file on open.

`app.json`:

```json
{
  "useMarkdownLinks": false,
  "newLinkFormat": "shortest",
  "alwaysUpdateLinks": false,
  "propertiesInDocument": "source",
  "attachmentFolderPath": "attachments",
  "newFileLocation": "current",
  "userIgnoreFilters": [
    "templates/records/",
    "tools/",
    "attachments/"
  ]
}
```

`templates.json`: `{"folder": "templates/records"}` in the same format.

- [ ] **Step 1: Write the failing tests:**

```python
# AC: CE-03.2
def test_templates_plugin_points_at_record_templates():
    assert read_json(".obsidian/templates.json") == {"folder": "templates/records"}
    plugins = read_json(".obsidian/core-plugins.json")
    assert plugins["templates"] is True and all(isinstance(v, bool) for v in plugins.values())
    assert set(plugins) == set(OBSIDIAN_1_14_CORE_PLUGINS)

# AC: CE-03.2
def test_links_are_bare_id_wikilinks_and_front_matter_stays_source():
    app = read_json(".obsidian/app.json")
    assert app["useMarkdownLinks"] is False and app["newLinkFormat"] == "shortest"
    assert app["alwaysUpdateLinks"] is False and app["propertiesInDocument"] == "source"
    assert app["attachmentFolderPath"] == "attachments" and app["newFileLocation"] == "current"
    assert app["userIgnoreFilters"] == ["templates/records/", "tools/", "attachments/"]

def test_obsidian_files_use_obsidians_own_format():
    # each of the three files: text == json.dumps(json.loads(text), indent=2) (no trailing \n)

def test_only_shared_settings_are_tracked():
    assert sorted(git("ls-files", ".obsidian").split()) == [
        ".obsidian/app.json", ".obsidian/core-plugins.json", ".obsidian/templates.json"]
```

- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Write the three files.**
- [ ] **Step 4: Run (`git add -A` first).** Expected: PASS.
- [ ] **Step 5: Commit** — `feat(content): add Obsidian settings for templates and id links`,
  `Refs: CE-03.2`.

---

### Task 7: VS Code snippets

**ACs:** CE-03.2

**Files:**
- Create: `tools/repo_checks/repo_checks/snippets.py`, `.vscode/theos.code-snippets`
- Test: `tools/repo_checks/tests/test_snippets.py`

**Interfaces:**
- Consumes: `parse_record` (Task 3), `validate_front_matter`, `version_parent_problems`,
  `KIND_NAMES`, `SAMPLES`, `IN_WORLD_TOPIC` (Task 4), `EXPECTED_FIELDS`, `render_template`
  (Task 5).
- Produces:
  - `@dataclass(frozen=True) class Snippet: name: str; prefix: str; scope: str; body: tuple[str, ...]; description: str`
  - `load_snippets(path: Path) -> list[Snippet]` (plain `json.loads`: the file is strict JSON,
    with no comments and no trailing commas).
  - `expand(body: Sequence[str], *, filename_base: str, directory: str) -> str`: joins the lines
    with `\n`, then resolves the following. Anything else raises `UnsupportedSnippetSyntax`.
    - Body escapes `\$`, `\}`, `\\`.
    - `$TM_FILENAME_BASE`, `${TM_FILENAME_BASE}` and `$TM_DIRECTORY`.
    - `${VAR/regex/format/flags}`. In the regex part `\/` becomes `/` and every other backslash
      stays. Applied with replace-first semantics (no match leaves the value unchanged). The
      format supports `$n`, `${n}` and `${n:+text}` (literal text, newlines allowed).
    - `${n:default}`, whose default may contain variables.
    - `${n|a,b|}`, which takes the first choice.
    - `$n` / `${n}` tab stops, which become empty.

Snippet rules:
- One snippet per kind. Name `Theos <kind>`, prefix `theos-<kind>`, scope `markdown`.
- Same front matter fields, order and body line as the template, with these lines:
  - `id: $TM_FILENAME_BASE` and `title: ${1:$TM_FILENAME_BASE}`.
  - Placeholders become tab stops with the spec §4.5 value as default.
  - Enums become choices, fail-safe default first: `${n|never,hinted,suspected,confirmed|}`,
    `${n|hinted,suspected,confirmed|}`, `${n|draft,canon|}` (canon first for equation),
    `${n|T1,T2,T3|}`, `${n|dialogue,description|}`, `${n|unrated,keep,kill,mix|}`.
  - The body line is the last tab stop, then `$0`.
- Parent derived from the folder, for floor, location, faction, npc, and `concept`/`map` of
  versions (regex text after JSON decoding):
  `${TM_DIRECTORY/^(?:.*[\/\\])?(?:(world|people|things|lore|concepts|maps)|([^\/\\]+))$/${1:+todo}$2/}`.
  - Band: `parent: tower`.
  - Floor number: `${TM_FILENAME_BASE/^f0*([0-9]+).*$/$1/}`.
  - Topic: one body line,
    `track: ${TM_FILENAME_BASE/^.*-(explaining|in-world)$/$1/}${TM_FILENAME_BASE/^(.*)-in-world$|^.*$/${1:+\ncounterpart: }$1${1:+-explaining}/}`.
    The `\n` is a real newline in the decoded text. The second transform adds the `counterpart`
    line only for in-world ids, and adds nothing (no blank line) for explaining ids.
- Regexes use only `(?:…)`, groups, alternation, `[…]` classes, `*`, `+`, `?`, `.`, `^`, `$`:
  constructs that behave the same in JavaScript and Python.

- [ ] **Step 1: Write the failing tests:**

```python
# AC: CE-03.2
def test_one_snippet_per_kind():
    snippets = load_snippets(repo_root() / ".vscode" / "theos.code-snippets")
    assert sorted(s.prefix for s in snippets) == sorted(f"theos-{k}" for k in KIND_NAMES)
    assert all(s.scope == "markdown" for s in snippets)

# AC: CE-03.2
@pytest.mark.parametrize("kind", KIND_NAMES)
@pytest.mark.parametrize("root", ["D:\\Projects\\tower-of-theos", "/home/jann/tower-of-theos"])
def test_snippet_gives_valid_front_matter(kind, root):
    # path = SAMPLES[kind].path joined to root with the root's separator
    # record = parse_record(expand(body, filename_base=stem, directory=dir))
    # validate_front_matter == [] and version_parent_problems == []
    # record.body.strip() == parse_record(render_template(template_text(kind), stem)).body.strip()

# AC: CE-03.2
def test_snippet_derives_values():
    # for both roots: floor parent "hearth", number 4; location parent "f04-choir-hall";
    # faction parent "hearth-folk"; npc parent "fates-cult"; band parent "tower";
    # concept-version concept "fates-shrine"; map-version map "f04-terrain";
    # every other field == EXPECTED_FIELDS[kind] value

def test_in_world_topic_gets_counterpart():
    # IN_WORLD_TOPIC: track "in-world", counterpart "spirit-magic-explaining", valid;
    # the explaining sample has no counterpart key and no blank line in front matter

def test_floor_number_edges():
    # floor snippet with stems "f100-summit" -> number 100, "f09-x" -> 9, "f00-x" -> 0 (invalid, reported)

def test_parent_is_todo_directly_in_kind_folder():
    # npc at people/mother-ilse.md, floor at world/f04-x.md, map-version at maps/f04-terrain-v1.md -> "todo"

def test_expander_rejects_unsupported_syntax():
    # "${CLIPBOARD}", "${1/(a)/${1:/upcase}/}", "${1:?a:b}" raise UnsupportedSnippetSyntax
```

- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement `snippets.py` and write `.vscode/theos.code-snippets`.**
- [ ] **Step 4: Run (`git add -A` first).** Expected: PASS.
- [ ] **Step 5: Commit** — `feat(content): add VS Code snippets for every kind`, `Refs: CE-03.2`.

---

### Task 8: Root README

**ACs:** CE-01.4

**Files:**
- Modify: `README.md` (today one line, `# tower-of-theos`)
- Test: `tools/repo_checks/tests/test_root_readme.py`

**Interfaces:**
- Consumes: `KIND_NAMES`, `KIND_FOLDERS`, `ID_PATTERNS` (Task 4), `repo_root()` (Task 1).

Content per spec §4.4, with level-2 headings exactly:
1. `## Where each kind lives`
2. `## System canon`
3. `## Binaries`
4. `## Local disk`
5. `## Making a record`
6. `## Checks`

Other content rules:
- A short intro says what the repo is: the content project now, the game and the map toolset
  later.
- The kind table has columns `Kind | Folder | Id`; each row starts with `` | `<kind>` | `` and its
  folder cell contains `KIND_FOLDERS[kind]`.
- Link to the data model:
  `https://github.com/FPGSchiba/theos-editor/blob/main/docs/specs/data-model.md`.
- No Confluence URLs.

- [ ] **Step 1: Write the failing tests:**

```python
# AC: CE-01.4
def test_readme_says_where_each_kind_lives():
    rows = table_rows(section("Where each kind lives"))
    for kind in KIND_NAMES:
        row = rows[kind]
        assert KIND_FOLDERS[kind] in row.folder, kind

# AC: CE-01.4
def test_readme_says_system_canon_stays_in_confluence():
    assert "Confluence" in section("System canon") and "canon/formalism" in section("System canon")

# AC: CE-01.4
def test_readme_says_binaries_live_in_a_synced_folder():
    text = section("Binaries").lower()
    assert "binaries folder" in text and "sync" in text and "@sha256:" in text and "attachments/" in text

# AC: CE-01.4
def test_readme_says_clone_must_be_on_a_local_disk():
    assert "local disk" in section("Local disk").lower()

def test_readme_explains_making_records_and_checks():
    # "Making a record" mentions "Insert template" and "theos-"; "Checks" contains the test command
```

`section(title)` returns the text between `## <title>` and the next `## ` heading; it fails if
the heading is missing.

- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Write `README.md`.**
- [ ] **Step 4: Run all tests.** Expected: PASS, every test of the suite.
- [ ] **Step 5: Commit** — `docs(content): describe the content layout in the root README`,
  `Refs: CE-01.4`.

---

## After the tasks (theos-phase steps 4–6)

- Final review loop per `CLAUDE.md` over the whole `tower-of-theos` branch.
- Verify:
  - The full test command passes on Windows (local) and on both CI jobs of the PR.
  - `git status` in the worktree is clean.
  - Every AC has a passing tagged test: `grep -rn "# AC:" tools/repo_checks/tests`, compared
    with the AC list. `cargo xtask` does not exist yet (P00), so `ac-trace` cannot run.
- PR in `tower-of-theos`:
  - title `feat(content): content layout and templates (P01)`;
  - body with theos-editor's template sections;
  - a "Manual checks for Jann" list: open the vault in Obsidian 1.14 and see `git status` stay
    clean; make one record with *Insert template*; make one with a VS Code snippet. Most useful are
    an in-world topic and a floor.
  - It waits for Jann.
- theos-editor companion PR: this plan plus `docs/status.md` (`in-progress`, then `pr-open` with
  the build PR link).
