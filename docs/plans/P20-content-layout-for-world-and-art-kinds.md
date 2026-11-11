# P20 · Content layout for world and art kinds — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the `tower-of-theos` layout of P01 with the world and art kinds of umbrella
version 2: folders, READMEs, templates, snippets, `theos.toml`, two agent guides, and tests that
prove CE-55 and CE-56 on Windows and Linux.

**Architecture:** Plain files in the content repo, checked by the existing Python test project
`tools/repo_checks`. P20 changes three things in that project:
- its kind tables follow data model v2, with enum values keyed by kind and field;
- `markdown-it-py` parses Markdown structure;
- two small test-side checkers verify the guides' worked examples: pin and camera lines, and
  quest steps with their flow.

Nothing generates the content files; the tests read what is committed.

**Tech Stack:** Python 3.12, uv 0.13.6, pytest, `markdown-it-py` 4.x, git CLI, GitHub Actions
(`ubuntu-latest`, `windows-latest`, existing workflow).

**Spec:** `docs/specs/phases/P20-content-layout-for-world-and-art-kinds.md` (theos-editor), which
extends `docs/specs/phases/P01-content-layout-and-templates.md`. Read both before any task. This
plan cites them as "spec §N" and "P01 §N".

**Where the work happens**
- **Build:** `D:\Projects\tower-of-theos\.worktrees\P20-content-layout-for-world-and-art-kinds`,
  branch `phase/P20-content-layout-for-world-and-art-kinds`, called `<repo>` below. Every path in
  the tasks is relative to `<repo>`.
- **This plan and `docs/status.md`:** theos-editor, branch
  `phase/P20-content-layout-for-world-and-art-kinds`, worktree
  `.worktrees/P20-content-layout-for-world-and-art-kinds`.
- **Test command,** from `<repo>`: `uv run --project tools/repo_checks --locked pytest tools/repo_checks -q`.
  A single test: append `-k <name>`.

## Global Constraints

Umbrella (`docs/specs/00-umbrella.md` §2, version 2), verbatim. G17–G19 were added after
CLAUDE.md's "G1–G16" was written, and are copied too:

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
- **G19** Positions on maps are fractions of a map version's frame (x east, y south, origin
  north-west); metres are derived from the frame. Pins and cameras live in the map record's body,
  never on the placed record (`docs/specs/data-model.md` §9.6–9.7).

Phase ACs (umbrella §3 P20), verbatim:

- CE-55.1 `enemies/` and `art/` exist with READMEs; quests live in `story/quests/` and spawns under
  their floor or location; the root README's table lists `enemy`, `spawn`, `quest` and `style`;
  `theos.toml` enables the part `atlas`.
- CE-55.2 One template and one VS Code snippet per new kind; a record made from each has valid
  front matter; the map template has empty `## Pins` and `## Cameras` sections.
- CE-55.3 `repo_checks` cover the new folders, templates and snippets on Ubuntu and Windows.
- CE-56.1 `tools/guides/place-on-the-atlas.md` and `tools/guides/write-a-quest.md` explain pins,
  cameras and quest steps as `docs/specs/data-model.md` §9.5–9.7 define them, each with one worked
  example marked as synthetic.

Phase-specific (spec and CLAUDE.md), one line each:

- **Commits** follow theos-editor `CONTRIBUTING.md` §2:
  - Conventional Commits with scope `content`;
  - type `test` for test-only changes, `feat` for content files, `docs` for READMEs and guides,
    `build` for the dependency;
  - footers `Refs: <AC ids>`, `Phase: P20`, `Claude-Session: a32ce4c4-6ee2-41bd-a5fa-75f0b8f8b00f`
    and `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Text files:** every file written is UTF-8 with LF, also on Windows, and ends with exactly one
  `\n`.
- **No lore:**
  - nothing in a README, template or guide invents canon;
  - test samples of the new kinds come from the data model's synthetic world-and-art example
    (spec §4.7);
  - guide examples use only reserved ids (spec §4.6, §5);
  - no content records outside test code and guide examples.
- **AC tags:** a test proves an AC with `# AC: <id>[, <id>]` on the line directly above the test
  function (P01 convention).
- **Tests** need no network once `uv sync` has run, and run unchanged on Windows and Linux.
- **The main checkout** of `tower-of-theos` (`D:\Projects\tower-of-theos`) is never touched; all
  work is in `<repo>`.
- **Engineering directives** (theos-editor `CLAUDE.md`):
  - small modules with one job;
  - use `markdown-it-py` for Markdown structure, never hand-split Markdown in new code (P01's
    line-based README helpers stay);
  - regexes only for our own line grammars;
  - the stdlib (`graphlib`) for the step graph.
- **Prose** in READMEs and guides: plain English and short sentences, like P01's READMEs.

## Review Focus

Inputs the spec implies, beyond what the ACs name, most likely to bite first:

1. **The same field name in different kinds.** `type` is an enum on quest and text on thing;
   `state` is draft/locked on style and T1/T3 on concept-version; `role` is an enum on enemy and
   text on npc. Each kind's values are checked against its own table. Test: Task 2
   `test_enum_values_depend_on_the_kind`.
2. **The spawn snippet placed directly in `world/`, under a floor and under a location**, with
   `/` and `\` directories. The parent comes out as `todo`, the floor id, or the location id. Test:
   Task 4 `test_spawn_parent_follows_its_folder`.
3. **Pin and camera lines in near-miss forms.** All of these are rejected:
   - `0.5,0.5` without a space, or `-` instead of `·`;
   - `[[id|text]]`, or x = `1.2`;
   - `fov 0`.

   These are accepted: `1`, `1.0`, `feature` together with `r`, and neither of them. Test: Task 6
   `test_pin_line_forms` and `test_camera_line_forms`.
4. **Step attribute lines after a blank line, `->` instead of `→`, and an `after::` step with no
   arrow into the join.** Each is a problem with its step. Test: Task 7
   `test_attribute_lines_must_follow_the_heading`, `test_outcome_needs_the_arrow_character`,
   `test_after_needs_an_arrow_into_the_join`.
5. **Example ids at the edges of the reserved block.** `Q-9900`, `Q-10000`, `samples-x` and
   `f05-sample-x` are not reserved; `Q-9901`, `Q-9999`, `sample-x` and `f04-sample-x` are. Test:
   Task 8 `test_reserved_id_edges`.

---

## File structure (`<repo>`)

| Path | Responsibility | Task |
|---|---|---|
| `tools/repo_checks/pyproject.toml`, `uv.lock` | `markdown-it-py` dependency | 1 |
| `tools/repo_checks/repo_checks/markdown.py` | Markdown blocks and sections via `markdown-it-py` | 1 |
| `tools/repo_checks/repo_checks/kinds.py` | Kind tables, `(kind, field)` enums, field checks of data model v2 | 2, 4 |
| `.vscode/theos.code-snippets` | `theos-template` purpose choice (2); four new snippets (4); map body (5) | 2, 4, 5 |
| `theos.toml`, READMEs, `.gitkeep`s, `tools/repo_checks/repo_checks/layout.py` | Layout and v2 READMEs (spec §4.1–§4.3) | 3 |
| `tools/repo_checks/repo_checks/samples.py`, `expected.py`, `templates/records/{enemy,spawn,quest,style}.template.md` | The four new kinds | 4 |
| `templates/records/map.template.md` | Empty Pins and Cameras | 5 |
| `tools/repo_checks/repo_checks/atlas.py` | Pin and camera lines, map body checks | 6 |
| `tools/repo_checks/repo_checks/quests.py` | Steps, attributes, flow, outcome sections | 7 |
| `tools/repo_checks/repo_checks/guides.py` | Guide shapes, example records, reserved ids | 8 |
| `tools/guides/place-on-the-atlas.md` | Atlas guide | 8 |
| `tools/guides/write-a-quest.md` | Quest guide | 9 |

Tests go in `tools/repo_checks/tests/`, one file per module as in P01: `test_markdown.py`,
`test_atlas.py`, `test_quests.py` and `test_guides.py` are new, and the others are extended.

---

### Task 1: Markdown structure through `markdown-it-py`

**ACs:** none directly (supports CE-55.2, CE-56.1). Untagged tests.

**Files:**
- Modify: `tools/repo_checks/pyproject.toml`, `tools/repo_checks/uv.lock` (via `uv add`)
- Create: `tools/repo_checks/repo_checks/markdown.py`
- Test: `tools/repo_checks/tests/test_markdown.py`

**Interfaces:**
- Produces, in `markdown.py`:
  - `@dataclass(frozen=True) class Block`, with fields:
    - `kind: Literal["heading", "paragraph", "item", "fence"]`;
    - `text: str`: the inline content, or the content for a fence;
    - `line: int`: the 0-based first source line;
    - `level: int = 0`: headings only;
    - `info: str = ""`: fences only.
  - `blocks(text: str) -> list[Block]`: top-down document order. List items give their inline
    text, tight or loose. Nested lists are flattened.
  - `@dataclass(frozen=True) class Section: title: str; line: int; blocks: tuple[Block, ...]`
  - `sections(text: str, level: int = 2) -> list[Section]`: every heading of that level, with the
    blocks up to the next heading of the same or a higher level. Deeper headings stay inside as
    blocks.
  - `title_and_lead(text: str) -> tuple[str | None, str | None]`:
    - the text of the first block if it is an `h1`, else `None`;
    - the text of the next block if it is a paragraph, else `None`.

- [ ] **Step 1:** `uv add --project tools/repo_checks "markdown-it-py>=4.3,<5"`. Expect
  `pyproject.toml` `dependencies` to list it and `uv.lock` to update. Then run the test command:
  all existing tests pass.
- [ ] **Step 2: Write the failing tests** in `test_markdown.py`:
  - `test_blocks_in_order`: `"# T\n\nLead.\n\n## A\nk:: v\n\n- [[x]] · 0.1, 0.2\n\n```markdown a/b.md\n---\n```\n"`
    gives kinds `[heading, paragraph, heading, paragraph, item, fence]`. The fence has
    `info == "markdown a/b.md"`, and the paragraph after `## A` has `line == 5`.
  - `test_sections_split_at_their_level`: two `##` sections, one with a `###` inside. The inner
    heading is a block of the first section, and the titles are `["A", "B"]`.
  - `test_heading_inside_a_fence_is_not_a_section`: a fence holding `## Step 1 — x` gives no
    section.
  - `test_title_and_lead`: `"# T\n\nLead line.\n\n## A\n"` → `("T", "Lead line.")`;
    `"Lead\n# T\n"` → `(None, None)`; `"# T\n\n- item\n"` → `("T", None)`.
- [ ] **Step 3: Run** `… -k test_markdown`. Expect FAIL (module missing).
- [ ] **Step 4: Implement `markdown.py`** over `MarkdownIt("commonmark").parse(text)`. Block lines
  come from `token.map[0]`.
- [ ] **Step 5: Run** the full test command. Expect PASS.
- [ ] **Step 6: Commit** — `build(content): parse Markdown structure with markdown-it-py`,
  `Refs: CE-55.2, CE-56.1`.

---

### Task 2: Enum values per kind and field; data model v2 fields of existing kinds

**ACs:** none directly (prepares CE-55.2). The snippet test changed here keeps its `CE-03.2` tag.

**Files:**
- Modify: `tools/repo_checks/repo_checks/kinds.py`, `.vscode/theos.code-snippets`
  (`Theos template` only)
- Test: `tools/repo_checks/tests/test_kinds.py`, `tools/repo_checks/tests/test_snippets.py`

**Interfaces:**
- Produces, in `kinds.py` (replacing P01's field-keyed `ENUMS`):
  - `TIERS: tuple[str, ...] = ("T1", "T2", "T3")`
  - `ENUMS: dict[tuple[str, str], tuple[str, ...]]`, keyed `(kind, field)`, with the first value
    as the snippets offer it. Entries now:
    - `(k, "status")`: `("draft", "canon")` for every kind;
    - `("band", "tier")` and `("clue", "gate")`: `TIERS`;
    - `("concept-version", "state")`: `("T1", "T3")`;
    - `("topic", "track")`: `("explaining", "in-world")`;
    - `("truth", "target")`: `("never", "hinted", "suspected", "confirmed")`;
    - `("clue", "moves_to")`: `("hinted", "suspected", "confirmed")`;
    - `("belief", "accuracy")`: `("false", "partial", "close")`;
    - `("template", "purpose")`: `("dialogue", "description", "prompt")`;
    - `("concept-version", "rating")` and `("map-version", "rating")`:
      `("unrated", "keep", "kill", "mix")`.
  - The lens `tier` checks against `TIERS` directly.
  - New field checks on existing kinds:
    - concept `type`: optional text;
    - map-version `frame`: optional, checked by `_frame`. It must be a map whose keys come from
      `size_m`, `parent`, `origin_m`. `size_m` is required: a list of two positive numbers (`int`
      or `Decimal`). `parent` (a map-version id) and `origin_m` (a list of two non-negative
      numbers) come both or neither.
  - `validate_front_matter` and `version_parent_problems` keep their signatures.
- Changes the `Theos template` snippet line to `purpose: ${2|dialogue,description,prompt|}`, with
  the same tab-stop number as today.

- [ ] **Step 1: Write the failing tests** in `test_kinds.py`:
  - `test_enum_values_depend_on_the_kind`: `minimal("thing")` with `type: "artifact"` → `[]`;
    `minimal("concept-version")` with `state: "draft"` → a problem naming `state`;
    `minimal("template")` with `purpose: "prompt"` → `[]`. Task 4 extends this test with quest,
    style and enemy.
  - `test_concept_type_is_optional_text`: `minimal("concept")` with `type: "key-art"` → `[]`.
  - `test_map_version_frame`: valid with `{"size_m": [24000, 24000]}`, and with
    `{"size_m": [Decimal("400.5"), 300], "parent": "f04-terrain-v1", "origin_m": [2400, 1200]}`.
    Each of these is a problem naming `frame`:
    - `{"size_m": [1]}`;
    - `{"size_m": [0, 10]}`;
    - `{"size_m": [10, 10], "parent": "f04-terrain-v1"}` (without `origin_m`);
    - `{"size_m": [10, 10], "origin_m": [0, 0]}` (without `parent`);
    - `{"size_m": [10, 10], "colour": "red"}`;
    - `"big"`.
  - In `test_snippets.py`, change `test_choice_lists_offer_every_allowed_value_with_the_fail_safe_first`
    to `ENUMS[(kind, field)]`, and `test_every_enum_field_of_the_snippets_has_a_choice_list` to
    `(kind, field) in ENUMS`. Keep the `topic`/`track` exception.
- [ ] **Step 2: Run.** Expect FAIL (key errors, unknown fields `type` and `frame`, `prompt`
  rejected).
- [ ] **Step 3: Implement** the `kinds.py` changes and the snippet line.
- [ ] **Step 4: Run** the full test command. Expect PASS: `prompt` is now among the template
  snippet's choice options, and `test_every_option_of_every_choice_gives_valid_front_matter`
  covers it.
- [ ] **Step 5: Commit** — `test(content): key enum values by kind and field, add v2 fields`,
  `Refs: CE-55.2`.

---

### Task 3: Layout, `theos.toml` and READMEs

**ACs:** CE-55.1, CE-55.3.

**Files:**
- Modify: `theos.toml`, `README.md`, `world/README.md`, `people/README.md`, `things/README.md`,
  `story/README.md`, `concepts/README.md`, `maps/README.md`, `templates/README.md`,
  `tools/README.md`, `tools/repo_checks/repo_checks/layout.py`
- Create: `enemies/README.md`, `art/README.md`, `story/quests/.gitkeep`,
  `tools/comfy/pipelines/.gitkeep`
- Test: `tools/repo_checks/tests/test_layout.py`, `tools/repo_checks/tests/test_root_readme.py`

**Interfaces:**
- Produces: `TOP_LEVEL_FOLDERS` in `layout.py` now `(".theos", "world", "people", "things",
  "enemies", "lore", "story", "concepts", "maps", "art", "templates", "dialogue", "canon",
  "tools")`.
- READMEs (spec §4.3) follow P01's folder README format: one `## <kind>` section per kind with
  Location, Id, Nesting, Template and Optional fields, then `## Making a record`. Their contents:
  - `enemies/README.md`: the `enemy` section.
  - `art/README.md`: the `style` section, plus a `## Sections and inheritance` section and a
    `## Reference crops` section with the points of spec §4.3.
  - `world/README.md`: gains `## spawn` after `## location`, with "under its floor or location".
  - `story/README.md`: gains `## quest`, linking `../tools/guides/write-a-quest.md`.
  - The root README's table: rows in data model §6 order (enemy and spawn after thing, quest
    after script, style last). The folder cells are `` `enemies/` ``,
    `` `under its floor or location` ``, `` `story/quests/` `` and `` `art/` ``. The id cells are
    "a slug", or "`Q-` and at least three digits" for quest. The link list gains `enemies/` and
    `art/`, and one sentence points agents to `tools/guides/`.
  - The v2 alignment lines of spec §4.3 go into `concepts/`, `maps/`, `templates/`, `world/`,
    `people/`, `things/`, `enemies/` and `tools/` READMEs. Where an optional field is listed,
    follow P01's "Optional fields:" line.

- [ ] **Step 1: Write the failing tests:**
  - In `test_layout.py`:
    - `test_theos_toml_is_the_data_model_manifest` (tag `# AC: CE-01.1, CE-55.1`) expects
      `["codex", "atlas", "changes", "scripts", "graph", "concepts", "maps", "review", "mcp"]`;
    - `test_every_top_level_folder_has_a_readme` is retagged `# AC: CE-01.1, CE-55.1`;
    - `test_folder_tree_of_the_architecture_exists` adds `"story/quests"` and
      `"tools/comfy/pipelines"`, tag `# AC: CE-01.1, CE-55.1`;
    - `test_ci_workflow_exists_and_runs_the_locked_tests_on_both_systems` is retagged
      `# AC: CE-01.1, CE-55.3`;
    - new `test_world_and_story_readmes_name_the_new_places` (`# AC: CE-55.1`): the world README
      has a `## spawn` heading and the text "under its floor or location"; the story README has
      `## quest` and `story/quests/`.
  - In `test_root_readme.py`, new `test_readme_lists_the_world_and_art_kinds`
    (`# AC: CE-55.1`): the table rows for `enemy`, `spawn`, `quest` and `style` hold `enemies/`,
    `under its floor or location`, `story/quests/` and `art/` in the folder cell, and a non-empty
    id cell. `test_readme_says_where_each_kind_lives` keeps its tag; it covers the new kinds once
    Task 4 adds them to `KIND_NAMES`.
- [ ] **Step 2: Run.** Expect FAIL.
- [ ] **Step 3: Write the files.** Check every README statement against data model §6–§9 and
  spec §4.3. Write no example names beyond the `sample-…` ids P01's READMEs already use.
- [ ] **Step 4: Run** the full test command. Expect PASS.
- [ ] **Step 5: Commit** — `feat(content): add the world and art folders and READMEs`,
  `Refs: CE-55.1, CE-55.3`.

---

### Task 4: The four new kinds — tables, samples, templates, snippets

**ACs:** CE-55.2, CE-55.3.

**Files:**
- Modify: `tools/repo_checks/repo_checks/kinds.py`, `samples.py`, `expected.py`,
  `.vscode/theos.code-snippets`
- Create: `templates/records/enemy.template.md`, `spawn.template.md`, `quest.template.md`,
  `style.template.md`
- Test: `tools/repo_checks/tests/test_kinds.py`, `test_templates.py`, `test_snippets.py`

**Interfaces:**
- Consumes: `ENUMS` keyed by kind and field (Task 2); `sections`, `blocks` (Task 1).
- Produces, in `kinds.py`:
  - `KIND_NAMES`: 24 kinds in data model §6 order. `"enemy"` and `"spawn"` go after `"thing"`,
    `"quest"` after `"script"`, and `"style"` last.
  - `KIND_FOLDERS`: `enemy → "enemies/"`, `spawn → "under its floor or location"`,
    `quest → "story/quests/"`, `style → "art/"`.
  - `KIND_TOP_FOLDER`: `enemy → "enemies"`, `spawn → "world"`, `quest → "story"`,
    `style → "art"`.
  - `ID_PATTERNS["quest"] = ^Q-[0-9]{3,}$`; the other three kinds are slugs.
  - Field tables, from data model §7 with spec §4.4 order. `threat` is an integer 1–5 (a new
    `_threat` check).
    - enemy: `bands` opt links → band, `threat` opt, `role` opt enum.
    - spawn: `parent` req link → floor or location, `of` req links → enemy, faction, people or
      npc, `threat` opt, `density` opt enum.
    - quest: `status` req, `type` req enum, `giver` opt link → npc, faction or people, `where` req
      links → band, floor or location, `requires` opt links → quest.
    - style: `for` req links → tower, band, people or faction, `extends` opt link → style,
      `state` req enum, `refs` opt map of role → binary ref. A non-map, or a value that is not
      `path@sha256:<64 hex>`, is a problem.
  - `ENUMS` gains:
    - `("enemy", "role")`: `("swarm", "brute", "ranged", "elite", "boss")`;
    - `("spawn", "density")`: `("low", "medium", "high")`;
    - `("quest", "type")`: `("main", "trial", "side")`;
    - `("style", "state")`: `("draft", "locked")`.
  - `"quest"` joins the kinds whose `status` is required.
- Produces, in `samples.py`:
  - `Sample("enemy", "marsh-lurker", "enemies/marsh-lurker.md")`;
  - `Sample("spawn", "reed-marsh-lurkers", "world/hearth/f04-choir-hall/reed-marsh/reed-marsh-lurkers.md")`;
  - `Sample("quest", "Q-012", "story/quests/Q-012.md")`;
  - `Sample("style", "hearth-style", "art/hearth-style.md")`.
- Produces, in `expected.py`:
  - `"enemy": {}`;
  - `"spawn": {"parent": "todo", "of": ["todo"]}`;
  - `"quest": {"status": "draft", "type": "main", "where": ["todo"]}`;
  - `"style": {"for": ["todo"], "state": "draft"}`.
- Templates: front matter exactly as in `EXPECTED_FIELDS`, after `id: {{title}}`,
  `kind: <kind>`, `title: {{title}}`. One body line each, with the instruction of spec §4.4. The
  style line names `## Prompt`, `## Negative`, `## Palette`, `## Light` and `## Materials` inside
  backticks, says a section left out is inherited, and contains no heading.
- Snippets (spec §4.5), with descriptions "Create an enemy record", "Create a spawn record",
  "Create a quest record" and "Create a style record". Each body line equals its template's.
  - `theos-enemy`: no fields.
  - `theos-spawn`: `parent:` uses the same `$TM_DIRECTORY` transform as `theos-location`, then
    `of: [${2:todo}]`.
  - `theos-quest`: `status: ${2|draft,canon|}`, `type: ${3|main,trial,side|}`,
    `where: [${4:todo}]`.
  - `theos-style`: `for: [${2:todo}]`, `state: ${3|draft,locked|}`.

- [ ] **Step 1: Write the failing tests:**
  - In `test_kinds.py`:
    - `test_kind_tables_cover_the_20_kinds` becomes `test_kind_tables_cover_the_24_kinds`
      (`# AC: CE-55.3`), asserting `len(KIND_NAMES) == 24` and the same set equalities;
    - `test_data_model_examples_are_valid` gains the §14 enemy, spawn, quest and style examples
      as strings (front matter only; the style's `…` hashes become `H`, as P01 does);
    - `MINIMAL` gains:
      - `"enemy": {}`;
      - `"spawn": {"parent": "reed-marsh", "of": ["marsh-lurker"]}`;
      - `"quest": {"status": "draft", "type": "side", "where": ["f04-choir-hall"]}`;
      - `"style": {"for": ["hearth"], "state": "locked"}`;
    - `test_required_list_fields_must_not_be_empty` adds `("spawn", "of")`, `("quest", "where")`
      and `("style", "for")`;
    - `test_enum_values_depend_on_the_kind` adds:
      - `type: "artifact"` on quest → a problem naming `type`;
      - `state: "T1"` on style → a problem;
      - `role: "brute"` on enemy → `[]`;
      - `role: "keeper"` on enemy → a problem, while npc `role: "keeper"` → `[]`;
    - new `test_threat_is_one_to_five`: enemy and spawn `threat` of 1 and 5 are valid; 0, 6 and
      `"high"` are problems;
    - new `test_style_refs_are_binary_references`: `{"buildings": f"art/b.png@sha256:{H2}"}` is
      valid; `{"sky": "art/s.png"}` and `"x"` are problems;
    - new `test_quest_ids`: `Q-012` and `Q-0012` match; `Q-12` and `q-012` do not;
    - new `test_new_kinds_are_in_every_table` (`# AC: CE-55.3`): each of the four kinds is in
      `KIND_NAMES`, `KIND_FOLDERS`, `KIND_TOP_FOLDER`, `ID_PATTERNS`, `SAMPLES`, `EXPECTED_FIELDS`
      and `MINIMAL`; `"enemies"` and `"art"` are in `TOP_LEVEL_FOLDERS`.
  - In `test_templates.py`:
    - retag `test_one_template_per_kind` and `test_record_made_in_obsidian_has_valid_front_matter`
      as `# AC: CE-03.1, CE-55.2`;
    - new `test_style_template_body_has_no_heading` (`# AC: CE-55.2`): no block of the rendered
      style record's body is a heading;
    - `test_folder_readmes_name_their_templates` gets `# AC: CE-55.1`.
  - In `test_snippets.py`:
    - retag `test_one_snippet_per_kind`, `test_snippet_gives_valid_front_matter`,
      `test_every_option_of_every_choice_gives_valid_front_matter` and
      `test_snippet_derives_values` as `# AC: CE-03.2, CE-55.2`;
    - `test_snippet_derives_values` gains `"spawn": {"parent": "reed-marsh"}`;
    - new `test_spawn_parent_follows_its_folder` (`# AC: CE-55.2`), for both `ROOTS`:
      - `world/sample-spawn.md` → `todo`;
      - `world/hearth/f04-choir-hall/sample-spawn.md` → `f04-choir-hall`;
      - `world/hearth/f04-choir-hall/reed-marsh/sample-spawn.md` → `reed-marsh`.
- [ ] **Step 2: Run.** Expect FAIL.
- [ ] **Step 3: Implement** the tables, samples, expected fields, four templates and four snippets.
- [ ] **Step 4: Run** the full test command. Expect PASS.

  The P01 tests parametrised over `KIND_NAMES` now cover all 24 kinds: templates, snippets, choice
  options, folder READMEs and the root README table.
- [ ] **Step 5: Commit** — `feat(content): add enemy, spawn, quest and style templates and snippets`,
  `Refs: CE-55.2, CE-55.3`.

---

### Task 5: Map template and snippet with empty Pins and Cameras

**ACs:** CE-55.2.

**Files:**
- Modify: `templates/records/map.template.md`, `.vscode/theos.code-snippets` (`Theos map`)
- Test: `tools/repo_checks/tests/test_templates.py`, `tools/repo_checks/tests/test_snippets.py`

**Interfaces:**
- Consumes: `blocks` (Task 1), `render_template`, `expand`, `parse_record` (P01).
- Produces: a map body of the brief line, a blank line, `## Pins`, a blank line, then
  `## Cameras` (spec §4.4). The snippet body lines are the brief tab stop, `""`, `"## Pins"`,
  `""`, `"## Cameras"` and `"$0"`.

- [ ] **Step 1: Write the failing tests:**
  - In `test_templates.py`, `test_map_template_has_empty_pins_and_cameras` (`# AC: CE-55.2`):
    the rendered map record's body `blocks` have kinds and texts
    `[(paragraph, <brief>), (heading 2, "Pins"), (heading 2, "Cameras")]`, and nothing else.
  - In `test_snippets.py`, `test_map_snippet_has_empty_pins_and_cameras` (`# AC: CE-55.2`): the
    same, for the expanded `theos-map` snippet under both `ROOTS`.
- [ ] **Step 2: Run.** Expect FAIL.
- [ ] **Step 3: Change** the template and the snippet.
- [ ] **Step 4: Run** the full test command. Expect PASS. Also expect P01's
  `test_templates_hold_no_date_time_comments_or_crlf` to still pass: one final `\n`, no CR.
- [ ] **Step 5: Commit** — `feat(content): start maps with empty Pins and Cameras sections`,
  `Refs: CE-55.2`.

---

### Task 6: Pin and camera lines (test-side checker)

**ACs:** none directly (supports CE-56.1). Untagged tests.

**Files:**
- Create: `tools/repo_checks/repo_checks/atlas.py`
- Test: `tools/repo_checks/tests/test_atlas.py`

**Interfaces:**
- Consumes: `sections` (Task 1); `Value` (P01 `ryaml`).
- Produces:
  - `class LineError(ValueError)`.
  - `@dataclass(frozen=True) class Pin`:
    - `record: str`, `x: Decimal`, `y: Decimal`;
    - `feature: str | None`, `radius_m: Decimal | None`.
  - `@dataclass(frozen=True) class Camera`:
    - `id: str`, `x: Decimal`, `y: Decimal`, `h_m: Decimal`;
    - `look_x: Decimal`, `look_y: Decimal`, `look_h_m: Decimal`;
    - `fov: Decimal`, `portrait: bool`.
  - `parse_pin(text: str) -> Pin`. `text` is the list item without `- `. It must match
    `[[<id>]] · <x>, <y>`, optionally followed by ` · feature <slug>`, then optionally
    ` · r <n> m`, in that order. Numbers are `[0-9]+(\.[0-9]+)?`. The id must be a valid id of
    any kind (`ID_PATTERNS`). x and y must be from 0 to 1, and r above 0. Otherwise it raises
    `LineError`.
  - `parse_camera(text: str) -> Camera`. `text` must match
    `<slug> · <x>, <y> · h <n> m · look <x>, <y> · h <n> m · fov <n>`, optionally followed by
    ` · portrait`. Positions must be 0–1 and `fov` 1–179. Otherwise it raises `LineError`.
  - `map_problems(map_id: str, body: str, versions: Mapping[str, Mapping[str, Value]]) -> list[str]`.
    `versions` maps version ids to their front matter. The problems:
    - in `## Pins`, the first paragraph is not exactly one line `placed_on:: [[<id>]]` when items
      follow;
    - that id is not in `versions`, its `map` field is not `map_id`, or it has no `frame`;
    - an item fails `parse_pin`;
    - a record appears twice;
    - in `## Cameras`, an item fails `parse_camera`;
    - a camera id appears twice.

    Each problem names the map and the line. Empty sections give no problem.
  - `frame_problems(versions: Mapping[str, Mapping[str, Value]]) -> list[str]`: a
    `frame.parent` that is not a key of `versions`.

- [ ] **Step 1: Write the failing tests:**
  - `test_pin_line_forms`:
    - accepted, with their parsed values:
      - `[[thornstead]] · 0.759, 0.334 · feature city-1`;
      - `[[elder-vass]] · 0.455, 0.610`;
      - `[[reed-marsh-lurkers]] · 0.120, 0.840 · r 120 m`;
      - `[[x]] · 1, 0 · feature a-1 · r 5 m`;
      - `[[x]] · 1.0, 0.0`;
    - `LineError`:
      - `[[x]] · 0.5,0.5`;
      - `[[x]] - 0.5, 0.5`;
      - `[[x|X]] · 0.5, 0.5`;
      - `[[x]] · 1.2, 0.5`;
      - `[[x]] · 0.5, 0.5 · r 0 m`;
      - `[[x]] · 0.5, 0.5 · r 5 m · feature a`;
      - `x · 0.5, 0.5`.
  - `test_camera_line_forms`:
    - accepted:
      - `gate-a6 · 0.62, 0.71 · h 62 m · look 0.55, 0.64 · h 115 m · fov 66 · portrait`
        (`portrait` is `True`);
      - the `shrine-stair` line of data model §14 (`portrait` is `False`);
    - `LineError` for `fov 0`, `fov 180`, `look 1.5, 0.2`, a missing `h … m`, and an id `Gate`.
  - `test_map_problems` over a hand-written body and `versions`:
    - a clean body → `[]`;
    - each of these gives one problem:
      - `placed_on` naming an unknown version;
      - a version of another map;
      - a frameless version;
      - pins without `placed_on`;
      - a duplicate record;
      - a duplicate camera id;
      - a bad pin line;
    - a body with empty `## Pins` and `## Cameras` → `[]`.
  - `test_frame_parent_must_be_known`.
- [ ] **Step 2: Run.** Expect FAIL.
- [ ] **Step 3: Implement** `atlas.py`, with one compiled regex per line grammar.
- [ ] **Step 4: Run.** Expect PASS.
- [ ] **Step 5: Commit** — `test(content): check pin and camera lines of map bodies`,
  `Refs: CE-56.1`.

---

### Task 7: Quest steps and flow (test-side checker)

**ACs:** none directly (supports CE-56.1). Untagged tests.

**Files:**
- Create: `tools/repo_checks/repo_checks/quests.py`
- Test: `tools/repo_checks/tests/test_quests.py`

**Interfaces:**
- Consumes: `sections`, `blocks` (Task 1); `TIERS`, `ID_PATTERNS` (Task 2/4).
- Produces:
  - `ATTRIBUTES: tuple[str, ...]`: the 14 names of data model §9.5, in table order: `at`, `with`,
    `do`, `fights`, `dialogue`, `script`, `reveals`, `when`, `next`, `on`, `after`, `quest`,
    `ends`, `repeat`.
  - `@dataclass(frozen=True) class Step: number: int; name: str; line: int; attrs: dict[str, tuple[str, ...]]`.
    Values are kept raw, one entry per line.
  - `parse_steps(body: str) -> tuple[list[Step], list[str]]`, which returns the steps and the
    problems. These are problems:
    - a level-2 heading that is not `Step <n> — <name>` (em dash, number without leading zeros);
    - a duplicate number;
    - no step 1;
    - attribute lines not on the line directly after the heading;
    - an unknown key;
    - a key twice (only `on` may repeat);
    - a value that does not fit its shape:
      - `at` and `quest`: exactly one `[[id]]`;
      - `with`, `fights`, `dialogue` and `reveals`: `[[id]]` items joined by `, `;
      - `script`: `[[S-…#<n>]]` items joined by `, `;
      - `do`: non-empty text;
      - `when`: one of `TIERS`;
      - `next` and `after`: numbers joined by `, `;
      - `on`: `<slug> → <n>`;
      - `ends`: `success` or `failure`;
      - `repeat`: `true`.

    The attribute lines are the leading lines of the first paragraph after the heading that
    match `^[a-z]+:: `.
  - `arrows(steps: Sequence[Step]) -> dict[int, set[int]]`: from `next` and `on`.
  - `flow_problems(steps: Sequence[Step]) -> list[str]`, with these problems:
    - a `next`, `on` or `after` target that is no step;
    - a step in `after` without an arrow into the waiting step;
    - a step unreachable from 1;
    - a step without `ends` and without an outgoing arrow;
    - a reachable step from which no `ends` is reachable;
    - a cycle without a `repeat:: true` step, found as a `graphlib.CycleError` on the graph with
      the repeat steps removed.

    Every problem names its step numbers.
  - `outcome_problems(steps: Sequence[Step], dialogue_bodies: Mapping[str, str]) -> list[str]`:
    for each step with `on` and `dialogue`, each linked dialogue must be in `dialogue_bodies` and
    have a level-2 section titled with each outcome.

- [ ] **Step 1: Write the failing tests:**
  - `test_data_model_quest_parses`: the §14 Q-012 body. Four steps; step 2 has
    `on == ("pay → 3", "refuse → 4")`; `arrows == {1: {2}, 2: {3, 4}, 3: set(), 4: set()}`;
    `flow_problems == []`.
  - `test_attribute_lines_must_follow_the_heading`: a blank line between the heading and `at::`
    gives one problem naming step 1.
  - `test_outcome_needs_the_arrow_character`: `on:: pay -> 3` is a problem.
  - `test_unknown_and_repeated_keys`: `colour:: red` gives a problem, as does `at::` twice.
  - `test_bad_values`: `when:: T4`, `ends:: maybe`, `at:: [[a]], [[b]]`, `next:: two`,
    `script:: [[S-004]]` (no beat) and `repeat:: yes` each give one problem.
  - `test_dangling_targets`: `next:: 9` gives a problem.
  - `test_unreachable_step`: a step 3 nothing points to gives a problem.
  - `test_path_without_ending`: a step without `ends` and without arrows gives a problem.
  - `test_loop_needs_repeat`: 1→2→1 with an ending reachable gives a problem; with
    `repeat:: true` on step 2 → `[]`.
  - `test_after_needs_an_arrow_into_the_join`:
    - 1 `next:: 2, 3`, 2 `next:: 4`, 3 `next:: 4`, 4 `after:: 2, 3` with `ends` → `[]`;
    - dropping step 3's `next:: 4` gives a problem naming steps 3 and 4.
  - `test_outcome_sections`: a step with `on:: pay → 2`, `on:: refuse → 3` and
    `dialogue:: [[d]]`:
    - `d`'s body has `## pay` and `## refuse` → `[]`;
    - without `## refuse` → one problem;
    - `d` missing from `dialogue_bodies` → one problem.
- [ ] **Step 2: Run.** Expect FAIL.
- [ ] **Step 3: Implement** `quests.py`.
- [ ] **Step 4: Run.** Expect PASS.
- [ ] **Step 5: Commit** — `test(content): check quest steps, flow and outcome sections`,
  `Refs: CE-56.1`.

---

### Task 8: Guide checks and the atlas guide

**ACs:** CE-56.1.

**Files:**
- Create: `tools/repo_checks/repo_checks/guides.py`, `tools/guides/place-on-the-atlas.md`
- Modify: `tools/README.md` (only if Task 3's line needs the final file name)
- Test: `tools/repo_checks/tests/test_guides.py`

**Interfaces:**
- Consumes: `blocks`, `sections`, `title_and_lead` (Task 1); `parse_record`,
  `validate_front_matter`, `KIND_TOP_FOLDER` (P01, Task 4); `map_problems`, `frame_problems`,
  `parse_pin`, `parse_camera` (Task 6); `ATTRIBUTES` (Task 7).
- Produces, in `guides.py`:
  - `@dataclass(frozen=True) class GuideSpec`, with fields:
    - `name: str`, `title: str`;
    - `sections: tuple[str, ...]`: exact `##` titles in order;
    - `names: tuple[str, ...]`: strings the guide must contain;
    - `tools: tuple[str, ...]`: tool names the `How to propose` section must contain.
  - `GUIDES: dict[str, GuideSpec]`:
    - `"place-on-the-atlas"`:
      - title `"Place records on the atlas"`;
      - sections `("Maps and frames", "Pins", "Cameras", "How to propose", "Worked example (synthetic)")`;
      - names `("placed_on::", "feature <", "r <n> m", "h <n> m", "look", "fov", "portrait", "<map id>#<camera id>")`;
      - tools `("atlas_read", "atlas_place")`.
    - `"write-a-quest"`:
      - title `"Write a quest"`;
      - sections `("Quest records", "Steps", "Rules", "How to propose", "Worked example (synthetic)")`;
      - names: the 14 `f"{a}::"` of `ATTRIBUTES`, plus `"## Step <n> — <name>"`;
      - tools `("quest_flow", "quest_propose")`.
  - `guide_text(name: str) -> str`: reads `tools/guides/<name>.md` via `read_text`.
  - `shape_problems(text: str, spec: GuideSpec) -> list[str]`, with these problems:
    - line 1 is not `# <title>`, or no lead paragraph follows;
    - the `##` titles are not exactly `spec.sections`;
    - text starts with `---`;
    - a name is missing;
    - a tool is missing from `How to propose`.
  - `@dataclass(frozen=True) class ExampleRecord: path: str; fields: dict[str, Value]; body: str`.
  - `example_records(text: str) -> list[ExampleRecord]`: the fences in
    `Worked example (synthetic)` whose info is `markdown <path>`. A fence there with another info
    string raises `ValueError`. A path with `\` raises `ValueError`.
  - `RESERVED_ID = re.compile(r"^(sample(-[a-z0-9]+)+|f04-sample(-[a-z0-9]+)*|[TBCSQ]-99(0[1-9]|[1-9][0-9]))$")`.
  - `LINK_FIELDS = ("parent", "for", "picked", "map", "concept", "from", "giver", "where", "requires", "speaker", "of", "home", "extends", "bands")`.
  - `example_ids(records: Sequence[ExampleRecord], section_text: str) -> set[str]`, collecting:
    - each `id`;
    - every value or list item of a `LINK_FIELDS` field;
    - `frame.parent`;
    - every `[[target]]` in `section_text`, without `#…` or `|…`.
- Guide content (spec §4.6). Write `place-on-the-atlas.md`:
  - restate data model §9.6, §9.7 and §8's frame and features for a reader who knows nothing
    else, generator-neutral, and add no rule;
  - show the line grammars literally:
    `- [[<id>]] · <x>, <y> · feature <feature id> · r <n> m` and
    `- <camera id> · <x>, <y> · h <n> m · look <x>, <y> · h <n> m · fov <degrees> · portrait`;
  - write the How to propose text as spec §4.6 says.

  The worked example (spec §4.6, only reserved ids):
  - a floor map `sample-floor-terrain` for `f04-sample-floor`, picked version
    `sample-floor-terrain-v1` with `frame: size_m: [4000, 3000]`;
  - four pins of four kinds (a location, an npc, a spawn, a thing), one with a `feature` and one
    with `r`;
  - two cameras, one `portrait`;
  - a child map for a settlement, whose version's frame has `parent: sample-floor-terrain-v1` and
    `origin_m: [2400, 1200]`, and which holds one pin;
  - prose that works out one pin's fractions from metres and the child pin's projected position.

  Verify every number by hand in the commit message body.

- [ ] **Step 1: Write the failing tests** in `test_guides.py`:
  - `test_reserved_id_edges`: `sample-x`, `sample-floor-terrain-v1`, `f04-sample-floor`, `Q-9901`
    and `Q-9999` match. `Q-9900`, `Q-10000`, `samples-x`, `f05-sample-x`, `sample` and `Q-012` do
    not.
  - `test_example_records_need_a_path`: a worked-example fence with info `markdown` alone raises
    `ValueError`.
  - The guide test, `# AC: CE-56.1`, parametrised over the atlas guide here and over both guides
    from Task 9:
    - `test_guide_has_its_shape`: `shape_problems(guide_text(name), GUIDES[name]) == []`.
  - These are tagged `# AC: CE-56.1`:
    - `test_atlas_example_is_marked_synthetic`: the first paragraph of the worked example
      contains "synthetic" and "not canon";
    - `test_atlas_example_uses_reserved_ids_only`: every id of `example_ids` matches
      `RESERVED_ID`;
    - `test_atlas_example_records_are_valid`:
      - for each record, `validate_front_matter(fields, stem) == []`, where `stem` is the path's
        file stem;
      - the path's first folder is `KIND_TOP_FOLDER[kind]`;
      - a version sits in `maps/<map>/`;
    - `test_atlas_example_is_sound`: for every map record, `map_problems(...) == []`, and
      `frame_problems(...) == []`;
    - `test_atlas_example_shows_what_the_spec_asks`: these hold:
      - a picked version has `frame.size_m == [4000, 3000]`;
      - every map's `placed_on` is one of its `picked` versions;
      - there are ≥ 4 pins, one with a feature and one with a radius;
      - there are 2 cameras, one `portrait`;
      - a version has a `frame.parent`.
- [ ] **Step 2: Run.** Expect FAIL.
- [ ] **Step 3: Implement** `guides.py`, then write the guide.
- [ ] **Step 4: Run** the full test command. Expect PASS.
- [ ] **Step 5: Commit** — `docs(content): add the atlas guide for agents`, `Refs: CE-56.1`.

---

### Task 9: The quest guide

**ACs:** CE-56.1.

**Files:**
- Create: `tools/guides/write-a-quest.md`
- Test: `tools/repo_checks/tests/test_guides.py`

**Interfaces:**
- Consumes: `GUIDES`, `guide_text`, `shape_problems`, `example_records`, `example_ids`,
  `RESERVED_ID` (Task 8); `parse_steps`, `flow_problems`, `outcome_problems`, `ATTRIBUTES`
  (Task 7); `validate_front_matter` (Task 4).
- Guide content: restate data model §7 (Story, quest) and §9.5 for a reader who knows nothing
  else, with the 14 attributes in a table and the rules of §9.5, and add no rule. The worked
  example follows spec §4.6, with only reserved ids:
  - quest `Q-9901` with a giver and a `where`;
  - a dialogue record for the outcome branch, with one `## <outcome>` section per outcome,
    holding designer-voice placeholders and no in-world lines;
  - a fight, and a reveal of `C-9901` with `when:: T1`;
  - two steps opened by one `next::`, each with `next::` into the join step, which has `after::`;
  - a `success` and a `failure` ending.

  `script::`, `quest::` and `repeat::` appear with sample values in the attribute table only.

- [ ] **Step 1: Write the failing tests:**
  - Add `"write-a-quest"` to the parametrisation of `test_guide_has_its_shape`.
  - These are tagged `# AC: CE-56.1`:
    - `test_quest_example_is_marked_synthetic`;
    - `test_quest_example_uses_reserved_ids_only`;
    - `test_quest_example_records_are_valid`, as for the atlas example;
    - `test_quest_example_is_sound`: for the quest, `parse_steps` returns no problems,
      `flow_problems == []`, and `outcome_problems` with the example's dialogue bodies is `[]`;
    - `test_quest_example_shows_what_the_spec_asks`: these hold:
      - some step has ≥ 2 `on` lines and a `dialogue`;
      - some step has `fights`;
      - some step has `reveals` and `when`;
      - some `next` lists two steps;
      - some step has `after`;
      - the `ends` values include both `success` and `failure`.
- [ ] **Step 2: Run.** Expect FAIL.
- [ ] **Step 3: Write** the guide.
- [ ] **Step 4: Run** the full test command. Expect PASS.
- [ ] **Step 5: Commit** — `docs(content): add the quest guide for agents`, `Refs: CE-56.1`.

---

## After the tasks (theos-phase steps 4–6)

- **Final review loop** per theos-editor `CLAUDE.md`, over the whole `tower-of-theos` branch
  against `origin/main`. The reviewer also gets spec §4.6 and data model §9.5–9.7, to check the
  guides' prose for correctness and invented rules.
- **Verify:**
  - the full test command passes on Windows (local) and on both CI jobs of the build PR;
  - `git status` in `<repo>` is clean;
  - `grep -rn "# AC:" tools/repo_checks/tests` shows every one of CE-55.1, CE-55.2, CE-55.3 and
    CE-56.1;
  - `cargo xtask ac-trace --phase P20` in theos-editor reports the phase as built in
    tower-of-theos;
  - `cargo xtask gate` runs green on the theos-editor companion branch (docs only).
- **PR in `tower-of-theos`:**
  - title `feat(content): content layout for world and art kinds (P20)`;
  - body with theos-editor's template sections, including the CI run links for Ubuntu and Windows
    (CE-55.3);
  - a "Manual checks for Jann" list:
    - make a spawn under a floor with *Insert template* and with the snippet;
    - make a map and see the empty sections;
    - make a style and see no headings;
    - open both guides on GitHub;
  - it waits for Jann.
- **theos-editor companion PR:** this plan plus `docs/status.md` (`in-progress`, then `pr-open`
  with the build PR link), titled
  `docs(docs): plan and status of content layout for world and art kinds (P20)`.
