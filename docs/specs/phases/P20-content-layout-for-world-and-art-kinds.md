# P20 · Content layout for world and art kinds — phase spec

Status: waiting for Jann's approval · 10 Nov 2026 · repo of the build: `tower-of-theos`
Source: `docs/specs/00-umbrella.md` §3 P20 (umbrella version 2, #19). Scope and acceptance criteria
come from there; this spec decides how. It extends the layout of P01
(`docs/specs/phases/P01-content-layout-and-templates.md`), whose rules hold unless this spec says
otherwise.

## 1. Goal

Slice S8 is "the world on the map". After P20, Jann can write enemies, spawns, quests and styles in
`tower-of-theos` with Obsidian or VS Code, and every new map record starts with the `## Pins` and
`## Cameras` sections the atlas reads. Agents get two guides that teach the pin, camera and quest
step grammars of data model §9.5–9.7. The editor phases that give these kinds their schemas and
tools (P21, P22, P24–P27) and CE-75 (the style bible, in a private session) then find a layout and
templates ready.

P20 ships only layout, READMEs, templates, snippets, `repo_checks` and the two guides. It writes
no schema, no content record and no lore.

## 2. Acceptance criteria (verbatim from the umbrella)

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

## 3. Scope

**Goals**
- The content tree matches `docs/architecture.md` §2 for the world and art kinds.
- The P01 files match data model v2 everywhere v2 touched them, not only for the new kinds (Jann,
  10 Nov 2026). `repo_checks`' kind tables equal data model §6–§8 as they stand now.
- Templates and snippets for `enemy`, `spawn`, `quest` and `style` give valid records with as
  little typing as the tools allow.
- The two guides are correct, and their worked examples are checked by tests: they parse, and
  their flows and positions are sound (Jann, 10 Nov 2026).
- Everything is checked by tests on Windows and Linux.

**Non-goals**
- No schemas or editor code. Enemy, spawn and quest schemas belong to P21, style to P26, map body
  parsing to P22.
- No content records. The style bible is CE-75 (private session, after P20). The lore import is
  CE-04.
- No ComfyUI pipelines (P27, P28), no `theos_blockout` (P23), no `asset-workflow.md` guide (P28).
- No changes to the Obsidian settings, and no new Obsidian plugins.

**Anti-goals** (would make P20 a failure even if every test passes):
- invented lore in templates, READMEs, guides or test data;
- a template default that fails open: a style template whose empty sections would hide the
  sections it inherits (§4.4), or a style that starts `locked`;
- a guide that teaches something other than data model §9.5–9.7, or stores a position on the
  placed record (G19);
- an easy path for binaries into git (G10), for example reference crops copied into `art/`;
- a guide example that an agent copies into real content because it looks real.

## 4. Design

Every part names the tool that does the work (engineering directive 2):

| Part | Tool |
|---|---|
| Layout, READMEs, templates, snippets, guides | Plain files. P01's conventions apply. |
| Tests | pytest under uv, as in P01 (`tools/repo_checks/`). |
| TOML | `tomllib` (stdlib). |
| Restricted YAML front matter | P01's test checker `repo_checks/ryaml.py`, unchanged. |
| Markdown structure of guides, templates and examples: headings, fenced blocks, lists, paragraphs | `markdown-it-py` (CommonMark). Probed on 10 Nov 2026 with 4.3.0: fence info strings, step headings and `key:: value` lines come out as needed. It is a new test-only dependency in `uv.lock`. |
| Pin, camera and step line shapes | Regexes in test code. These grammars are our own (data model §9.5–9.7), so no crate exists for them. |
| Step graph (reachability, endings, loops) | Plain Python over a dict of edges. The stdlib suffices; no graph library. |
| Git facts (tracked, ignored, eol) | The `git` command line, as in P01. |

### 4.1 Tree

New or changed paths (P01's §4.1 tree stays):

```text
tower-of-theos/
  README.md                     table gains enemy, spawn, quest, style (§4.3)
  theos.toml                    parts gain "atlas" (§4.2)
  world/README.md               gains spawn; Look sections (§4.3)
  people/README.md  things/README.md   Look sections (§4.3)
  enemies/         README.md    new top-level folder, flat
  art/             README.md    new top-level folder, flat
  story/README.md               gains quest
  story/quests/    .gitkeep     new
  concepts/README.md  maps/README.md  templates/README.md  tools/README.md   v2 fields (§4.3)
  templates/records/            + enemy, spawn, quest, style templates; map template changed (§4.4)
  .vscode/theos.code-snippets   + 4 snippets; map and template snippets changed (§4.5)
  tools/guides/    place-on-the-atlas.md · write-a-quest.md (§4.6)
  tools/comfy/pipelines/  .gitkeep   new, empty until P27 and P28
  tools/repo_checks/            extended (§4.7)
```

- "Top-level folder" (P01 §4.1) now also means `enemies` and `art`; both get a README.
- Spawns get no folder of their own. A spawn sits under its floor or location, in the same folder
  as that place's child locations (`world/<band>/<floor>/` or `world/<band>/<floor>/<location>/`),
  as data model §6 says.
- `tools/comfy/pipelines/` gets a `.gitkeep`, like P01 did for `tools/comfy/workflows/`, so the
  tree matches architecture §2. `tools/theos_blockout/` is left to P23.

### 4.2 `theos.toml`

Exactly the example of data model §13:
`enabled = ["codex", "atlas", "changes", "scripts", "graph", "concepts", "maps", "review", "mcp"]`.

### 4.3 READMEs

Same format as P01's folder READMEs: one section per kind with location, id, nesting, template and
optional fields.

- **Root README.** The *Where each kind lives* table gains four rows:
  - `enemy`: `enemies/`, a slug;
  - `spawn`: `under its floor or location`, a slug;
  - `quest`: `story/quests/`, `Q-` and at least three digits;
  - `style`: `art/`, a slug.

  The link list names `enemies/` and `art/`. A short pointer says that agents' guides live in
  `tools/guides/`.
- **`enemies/README.md`**:
  - enemy records are flat in `enemies/`;
  - optional fields: `bands`, `threat`, `role`, `status`, `tags`;
  - the body says what the enemy is and what the folk believe it is, with an optional `## Look`;
  - tuning numbers (stats, spawn tables, loot) are not part of the schema.
- **`art/README.md`**:
  - style records are flat in `art/`;
  - required: `for` and `state`; optional: `extends`, `refs`, `status`, `tags`;
  - `state: locked` is set by Jann on purpose. A change to a locked style is high significance in
    review, and no agent tool locks or unlocks a style;
  - the body sections of data model §9.8 (`## Prompt`, `## Negative`, `## Palette`, `## Light`,
    `## Materials`, and optionally one per concept type);
  - a section the style leaves out is inherited from the style it `extends`, so an empty section
    must not be left in;
  - `refs` are binary references: their paths are relative to the binaries folder, not to this
    repo, and reference crops never go into git.
- **`world/README.md`** gains a `spawn` section:
  - location: under its floor or location;
  - required: `parent`, `of`; optional: `threat`, `density`, `status`, `tags`;
  - where a spawn is on a map is a pin in that map's body, never a field of the spawn;
  - a spawn no quest step fights is ambient.
- **`story/README.md`** gains a `quest` section:
  - quests are flat in `story/quests/`, with their steps inside the file;
  - optional fields: `giver`, `requires`, `tags`;
  - it links to `tools/guides/write-a-quest.md`.
- **v2 alignment of P01's READMEs:**
  - `concepts/README.md`: concept optional `type`;
  - `maps/README.md`: map bodies hold `## Pins` and `## Cameras` (link to the atlas guide);
    map-version optional `frame`;
  - `templates/README.md`: `purpose` is `dialogue`, `description` or `prompt` (prompt templates
    build briefs);
  - `world/`, `people/`, `things/` and `enemies/` READMEs: locations, people, factions, npcs,
    things and enemies may have a `## Look` section in designer voice, saying what a picture of
    the record must show (data model §9.9);
  - `tools/README.md` names `guides/` (agents' guides; `asset-workflow.md` arrives with P28),
    `comfy/pipelines/` (P27, P28) and `theos_blockout/` (P23).

### 4.4 Record templates (CE-55.2)

P01's template rules hold:
- named `<kind>.template.md`, `id: {{title}}`, `title: {{title}}`;
- the §5 fields first, then the kind's fields in table order;
- only required fields, no comments, no dates;
- the body is one instruction line, never lore.

Placeholders and fail-safe defaults are as in P01 §4.5. Where no fail-safe applies, the first
allowed value is the default.

| Kind | Fields after `id`, `kind`, `title` | Body (instruction, wording left to the builder) |
|---|---|---|
| enemy | – | what it is and what the folk believe it is; add `## Look` for what a picture must show |
| spawn | `parent: todo`, `of: [todo]` | what appears here and how; its place on a map is a pin in the map's body |
| quest | `status: draft`, `type: main`, `where: [todo]` | why the quest exists, in designer voice; add steps as `## Step 1 — <name>` headings (see `tools/guides/write-a-quest.md`) |
| style | `for: [todo]`, `state: draft` | name the sections briefs draw from, in backticks inside the sentence, and say that a section left out is inherited |
| map (changed) | `for: [todo]` (unchanged) | the brief line of P01, then a blank line, `## Pins`, a blank line, `## Cameras` |

- **Map.** "Empty" means each heading has nothing under it. `## Pins` gets no `placed_on::`
  line, because no version exists yet. The file still ends with exactly one `\n`.
- **Style.** The template holds no section headings. Data model §7 (Art) takes each section from
  the nearest style that has it, so an empty `## Palette` would blank the palette of every concept
  under that style. The instruction line names the sections inside backticks, so no heading is
  parsed. `state: draft` is both the first value and the fail-safe; locking is deliberate.
- **Quest.** `type: main` is the first allowed value. No guardrail depends on the type, so P01's
  rule applies. The snippet offers `main`, `trial` and `side`.
- **Enemy.** No fields are required, as for `tower` and `people`. Like the other kinds, its template
  gets no `## Look` heading: P01's one-line body rule holds, and the READMEs explain Look.

No other P01 template changes. In particular, the `template` kind keeps `purpose: dialogue`.

### 4.5 VS Code snippets (CE-55.2)

One snippet per new kind in `.vscode/theos.code-snippets`: `theos-enemy`, `theos-spawn`,
`theos-quest`, `theos-style`. They use P01 §4.7's syntax subset and derivations:
- `theos-spawn` derives `parent` from the folder, like `theos-location`, with `todo` for a file
  directly in a kind folder;
- `theos-quest` offers `status` (`draft`, `canon`) and `type` (`main`, `trial`, `side`) as choice
  lists;
- `theos-style` offers `state` (`draft`, `locked`);
- the other placeholders are tab stops.

Changed snippets:
- `theos-map` writes the same body as the map template, with its two empty sections;
- `theos-template` offers `purpose` as `dialogue`, `description`, `prompt`.

Each snippet's body equals its template's body, as P01's test already requires.

### 4.6 Guides for agents (CE-56.1)

Two Markdown files in `tools/guides/`. `tools/` is not a record folder (P01 §5), and Obsidian
already excludes it. The files have no front matter.

**Shape** (P27 serves these files as MCP prompts and lists them in `project_info`, CE-72.2):
1. line 1 is `# <title>`;
2. the next block is one paragraph that says what the guide is for (P27's prompt description);
3. level-2 sections, with exactly these headings, in this order:

| Guide | Sections |
|---|---|
| `place-on-the-atlas.md` · `# Place records on the atlas` | `## Maps and frames` · `## Pins` · `## Cameras` · `## How to propose` · `## Worked example (synthetic)` |
| `write-a-quest.md` · `# Write a quest` | `## Quest records` · `## Steps` · `## Rules` · `## How to propose` · `## Worked example (synthetic)` |

**Content.** The guides restate data model §9.5–9.7 and the fields they rest on. They add no rule
of their own.

- **Atlas guide:**
  - a map's layout lives in the map's body, never on the placed record (G19);
  - positions are fractions of the picked version's frame: x east, y south, origin north-west,
    0–1, with metres derived from `frame.size_m`;
  - the `## Pins` line grammar: `placed_on::`, link, `x, y`, `feature <id>`, `r <n> m`;
  - which kinds can be pinned (location, npc, spawn, thing);
  - the rules: at most once per map; a location or spawn belongs to the map's place or a place
    below it; pin once, on the most detailed map; projection onto parent maps is derived;
    `placed_on` that is not the picked version warns, and re-binding fixes it;
  - a version without a `frame` takes no pins;
  - the `## Cameras` line grammar: id, position, `h`, `look` and its height, `fov`, `portrait`;
    referred to as `<map id>#<camera id>`; blockout runs render cameras for pipelines;
  - it stays generator-neutral (G11): `frame` and `features` come from any map generator.
- **Quest guide:**
  - the quest fields: `type` with the meaning of main, trial and side; `giver`, `where`,
    `requires` and `status`;
  - the step grammar: heading, attribute lines directly after it, then prose; `Q-012#2`
    addressing; step 1 is the start;
  - all 14 attributes of the §9.5 table with their values;
  - every rule of §9.5: any-of arrows and the `after::` join; reachability; every path ends; loops
    need `repeat:: true`; no cycles through `quest::`; the lore rules run per path; outcome
    sections in dialogues;
  - the beat-style `reveals::` checks.
- **`## How to propose`** (Jann, 10 Nov 2026):
  - over the editor's MCP server, agents read with `atlas_read` or `quest_flow` and propose with
    `atlas_place` or `quest_propose` (umbrella CE-62.1, CE-59.1, CE-59.2);
  - a proposal becomes a review entry, and agents never write canon (G7);
  - the tools arrive with the editor's world kinds and atlas. Until the connected server offers
    them, an agent edits the record files on a branch and opens a pull request for review.

**Worked example.** One per guide, under `## Worked example (synthetic)`:
- The section opens with a sentence saying that every name in it is synthetic, not canon, and must
  not be copied into content.
- Ids are neutral. Slugs start with `sample-` (the floor is `f04-sample-floor`), and numbered ids
  come from the block 9901–9999 of their prefix (`Q-9901`, `C-9901`), which §5 reserves. They do
  not reuse the data model's illustration names, so no agent mistakes an example for canon.
- Texts are designer-voice placeholders. There are no in-world lines and no lore claims.
- Each example record is one fenced block whose info string is `markdown <repo path>`, such as
  ```` ```markdown story/quests/Q-9901.md ````. The tests read the path from it.
- **Atlas example:**
  - a floor map with its picked version, whose `frame` is `size_m: [4000, 3000]`;
  - pins of four kinds, one bound to a `feature` and one with a radius;
  - two cameras, one of them `portrait`;
  - a child map (a settlement inside the floor) whose version has
    `frame.parent`/`origin_m`;
  - prose that works out one pin's fractions from metres, and one child pin's projected position on
    the parent map.
- **Quest example:**
  - one quest with a `giver` and `where`;
  - a branch on dialogue outcomes (`on::`), with the dialogue record showing one `## <outcome>`
    section per outcome, as placeholders;
  - a fight (`fights::`) and a reveal (`reveals::` with `when::`);
  - two steps open at once (`next::` with two numbers), joined by `after::`. Each of the two steps
    has a `next::` arrow into the join step, and the join step lists both in `after::`. Data model
    §9.5 does not say whether `after::` is an arrow of its own or only a wait condition on the
    arrows. This form is valid under either reading; P21 settles which applies (§5);
  - a `success` and a `failure` ending.

  `script::`, `quest::` and `repeat::` appear in the attribute table with sample values.

### 4.7 Tests (`tools/repo_checks/`)

As in P01 §4.8: pytest under uv, `# AC: <id>` comments above tests, no network after the first
sync, CI on `ubuntu-latest` and `windows-latest` (existing workflow). Tests that already carry
`CE-03.x` and now also cover the new kinds carry both ids (`# AC: CE-03.1, CE-55.2`).

**Kind tables (data model v2).** `KIND_NAMES` has 24 kinds. `KIND_FOLDERS`, `KIND_TOP_FOLDER`,
`ID_PATTERNS` (quest `^Q-[0-9]{3,}$`), the samples, `EXPECTED_FIELDS` and the validator's field
tables all gain the four kinds:

- enemy: `bands`, `threat` 1–5, `role`;
- spawn: `parent` → floor or location, `of`, `threat`, `density`;
- quest: `status` required, `type`, `giver`, `where`, `requires`;
- style: `for`, `extends`, `state`, `refs` as a role → binary-ref map.

The data model v2 additions to existing kinds go in too:
- concept `type`;
- map-version `frame`: a map with `size_m` (two numbers) and, only together, `parent`
  (→ map-version) and `origin_m` (two numbers);
- template `purpose` gains `prompt`.

**Enum values are keyed by kind and field.** P01's `ENUMS` is keyed by field name. Three field
names now mean different things in different kinds:
- `type`: an enum on quest, free text on thing, location, concept and map;
- `state`: `T1`/`T3` on concept-version, `draft`/`locked` on style;
- `role`: an enum on enemy, free text on npc.

So the table becomes `(kind, field) → allowed values`, and the snippet tests read it the same way.

**Samples** for the new kinds are test inputs, as in P01, taken from the data model's synthetic
world-and-art example:
- enemy `marsh-lurker` (`enemies/marsh-lurker.md`);
- spawn `reed-marsh-lurkers` (`world/hearth/f04-choir-hall/reed-marsh/reed-marsh-lurkers.md`);
- quest `Q-012` (`story/quests/Q-012.md`);
- style `hearth-style` (`art/hearth-style.md`).

The data model's new examples that have front matter (the enemy, spawn, quest and style) join
P01's `test_data_model_examples_are_valid`, which checks front matter only. The map body fragment
of §14 has no front matter and does not go through the pin checker. Its `placed_on` version,
`f04-terrain-v1`, has no `frame` in the examples, while §8 says a version without one takes no
pins. That is listed under "Needs Jann" in `docs/status.md`, not worked around.

| AC | What the tests prove |
|---|---|
| CE-55.1 | `enemies/README.md` and `art/README.md` exist, are non-empty and name their templates; `story/quests/.gitkeep` exists; the world README's spawn section says "under its floor or location"; the root README's table has rows for `enemy`, `spawn`, `quest` and `style` with the folder of data model §6; `theos.toml` equals data model §13. |
| CE-55.2 | `templates/records/` holds exactly the 24 `<kind>.template.md` files; each new template, made into a record the way Obsidian does, has valid front matter by P01's four rules and exactly the fields of §4.4; one snippet per kind (24), each giving valid front matter for its sample path with `/` and `\` directories; every choice option is valid; `theos-spawn` derives its parent from the folder. The map template and the map snippet each parse (markdown-it-py) into the brief paragraph, then the heading `Pins` and directly the heading `Cameras`, with nothing after. The style template's body has no heading. |
| CE-55.3 | The CI workflow runs the locked test command on Ubuntu and Windows (P01's test, tagged with both ids). The new kinds are in every kind table, and the template and snippet tests are parametrised over all 24 kinds. The new folders are in the layout lists of `test_layout.py`. The PR links the green runs on both systems. |
| CE-56.1 | Both guide files exist, with the shape and headings of §4.6. The atlas guide names `placed_on::`, `feature`, `r <n> m`, `h`, `look`, `fov` and `portrait`. The quest guide names all 14 attributes of §9.5. The `## How to propose` sections name the tools of §4.6. Each worked example section has the synthetic sentence, and every id in it (front-matter `id` and link fields, `[[…]]` targets) is a `sample-…` or `f04-sample-…` slug or a numbered id from 9901 to 9999. Each example parses and is sound (below). |

**Example checks** (test code over the fenced blocks of the worked examples):
- Every example record: its front matter is valid by the four rules; its path's top folder matches
  its kind; its id equals the file stem.
- **Atlas example:**
  - `## Pins` holds one `placed_on::` naming a map-version of that map in the example, and that
    version has a `frame`;
  - every pin line matches `- [[<id>]] · <x>, <y>` with optional ` · feature <id>` and
    ` · r <n> m`, and x and y are decimals from 0 to 1;
  - no record appears twice on one map;
  - every camera line matches
    `- <slug> · <x>, <y> · h <n> m · look <x>, <y> · h <n> m · fov <n>`, with optional
    ` · portrait`; coordinates are 0–1, `fov` is from 1 to 179, and ids are unique on the map;
  - a `frame.parent` names an example map-version.
- **Quest example:**
  - step headings match `Step <n> — <name>` with unique numbers and a step 1;
  - attribute lines come directly after the heading, with keys from the §9.5 table and values of
    its shape;
  - `next::`, `on::` and `after::` name existing steps;
  - arrows are `next::` and `on::`; every step named in a step's `after::` has an arrow into that
    step (the join form of §4.6);
  - every step is reachable from step 1;
  - from every step a step with `ends::` is reachable, and every step without `ends::` has an
    outgoing arrow;
  - every cycle has a step with `repeat:: true`;
  - for a step with `on::` and `dialogue::`, the example dialogue has a level-2 section per
    outcome.

These checkers get negative tests (a pin at x 1.2, a duplicate camera id, a dangling `next::`,
an unreachable step, a loop without `repeat::`, a path without an ending, a missing outcome
section). A checker that accepts everything cannot pass them. They are test code, like P01's
YAML checker, not a second P21 or P22. They check only what the examples use.

### 4.8 How the build runs

As P01 §4.9. The build branch is `phase/P20-content-layout-for-world-and-art-kinds` in
`../tower-of-theos/.worktrees/P20-content-layout-for-world-and-art-kinds` (`.worktrees/` is
already ignored there). Commits follow `CONTRIBUTING.md` §2, with scope `content`, `Refs:` and
`Phase: P20`. The PR is `feat(content): content layout for world and art kinds (P20)`, uses
theos-editor's template sections, and waits for Jann.

The plan (`docs/plans/P20-content-layout-for-world-and-art-kinds.md`) and the phase state stay in
this repo, on a companion branch of the same name. Its PR,
`docs(docs): plan and status of content layout for world and art kinds (P20)`, touches no
protected path. Phase ACs are proven in `tower-of-theos`, and `ac-trace` reads the repo column of
umbrella §3, so it demands no tests here.

## 5. Interfaces other phases use

- **P21 (enemy, spawn, quest schemas):**
  - kind folders as in §4.1, and the quest id pattern `Q-` with at least three digits;
  - its schemas must accept the four templates as made in Obsidian, with placeholders dangling, and
    the quest guide's example;
  - a quest made from the template has no steps; P21 decides whether that is a finding;
  - the quest example writes a join as `next::` arrows into the join step plus `after::` on it,
    which is valid whether `after::` is an arrow or only a wait condition. P21's spec settles the
    semantics. If it rules out this form, the guide changes with it.
- **P22 (pins and cameras):**
  - a new map body holds `## Pins` and `## Cameras` with nothing under them; the parser must take
    empty sections and an absent `placed_on::`;
  - the atlas guide's example is a fixture to agree with. If P22's parser disagrees, the guide is
    fixed in `tower-of-theos`.
- **P24, P25:** use the quest and atlas examples as readable references. No contract beyond the
  data model.
- **P26 (style):**
  - styles live flat in `art/`, start `state: draft` and have no body sections;
  - an absent section inherits;
  - `refs` paths are relative to the binaries folder.
- **P27 (guides as MCP prompts):**
  - every file in `tools/guides/*.md` has the shape of §4.6: line 1 `# <title>`, then one
    description paragraph, then `##` sections, with no front matter;
  - the file stem is the guide's name; P27 maps it to a prompt name;
  - when P21 and P22 have shipped their tools, the "until then" sentence of `## How to propose`
    is dropped in a `tower-of-theos` change (P28 at the latest).
- **P28:** adds `tools/guides/asset-workflow.md` in the same shape, and pipelines in
  `tools/comfy/pipelines/`.
- **P23:** adds `tools/theos_blockout/`.
- **P14 (new content project, CE-38.2):** this layout is the reference for the tree it creates.
- **CE-75:** style records from the style bible land in `art/` as `art/README.md` describes.
- **Reserved:** P01's placeholders stay reserved. Guide example ids are never used for real
  records: slugs starting with `sample-` or `f04-sample-`, and the numbers 9901–9999 of every
  numbered prefix. Like an agent session's block (data model §3), this block is set aside, so
  real numbering never reaches it by accident.

## 6. Decisions

Decided by Jann (10 Nov 2026):
- **How to propose.** The guides teach the grammar and rules, and name the umbrella's MCP tools
  with the phases that deliver them. Until those tools exist, an agent edits record files on a
  branch for review.
- **Example checks.** `repo_checks` parse the worked examples and check them for soundness (§4.7),
  not only for structure.
- **v2 alignment.** P20 aligns all P01 files with data model v2, not only the new kinds.

Claude's calls inside that frame (Jann checks them in this review):
- **Style template.** It has no section headings, because an empty section would hide the
  inherited one. It starts at `state: draft`.
- **Quest default.** `type: main`, by P01's first-value rule.
- **Enemy template.** One body line; no `## Look` heading in any template (READMEs explain Look).
- **Guide examples.** Neutral ids (`sample-…`, the reserved block 9901–9999), not the data
  model's illustration names. The repo_checks samples still take the data model's synthetic
  world-and-art ids, as P01 did.
- **Join form.** The quest example writes joins as `next::` arrows plus `after::`, the form that
  holds under either reading of data model §9.5. P21 settles the semantics.
- **Data model examples.** Only the examples with front matter join the front-matter test. The
  frameless `placed_on` version of §14 goes to Needs Jann.
- **Guide shape.** No front matter; H1, then a description paragraph, then fixed `##` headings
  (for P27 and the tests).
- **Example paths.** The info string `markdown <path>` carries each example record's path.
- **Markdown parsing.** `markdown-it-py` as the Markdown parser of the new tests. P01's existing
  line-based README tests stay as they are.
- **Enum table.** Keyed by kind and field.
- **Pipelines folder.** `tools/comfy/pipelines/.gitkeep` now; `tools/theos_blockout/` left to
  P23.
- **Plan and status.** Kept on a companion branch in this repo, as for P01.
- **Status update.** The spec PR touches only the P20 row of `docs/status.md`. The open P04 spec
  PR #21 already sets P03 to done.

## 7. Risks

- **The test checkers drift from the real parsers** (P21, P22, P26). Mitigation: they follow the
  data model text, cover only what the examples use, and P21/P22 treat the guide examples as
  fixtures to agree with (§5).
- **The guides go stale** when the MCP tools ship, or if the data model changes. Mitigation: the
  `## How to propose` sentence is named in §5 for removal. A data model change is a protected-path
  PR that lists the guides as follow-ups.
- **Enum collisions** (`type`, `state`, `role`) would make P01's field-keyed table accept or offer
  wrong values. Mitigation: the `(kind, field)` table, with a test that a thing's
  `type: artifact` stays valid while a quest's `type: artifact` fails.
- **Obsidian inserts the map template's headings.** Two headings after the front matter go in as
  they are. Obsidian only rewrites front matter, and the template still goes into an empty note.
- **Guide examples copied into content.** Mitigation: the synthetic sentence, the `sample-` ids
  and the reserved block 9901–9999.
- **Reference crops copied into `art/`.** Mitigation: the README, plus P01's existing test that
  fails on any tracked binary file.
- **A new test dependency** (`markdown-it-py`). It is locked in `uv.lock`, CI already syncs with
  `--locked`, and it is pure Python.

## 8. Review Focus

Inputs most likely to break this phase:
- the same field name across kinds: `type` on quest vs thing, `state` on style vs
  concept-version, `role` on enemy vs npc;
- quest ids `Q-12` (too short), `Q-0012`, `q-012`;
- a spawn file directly in `world/`, under a floor, and under a location, with `/` and `\`
  directories, in the snippet derivation;
- the map template and snippet: nothing between `## Pins` and `## Cameras`, exactly one final
  `\n`, no CR on Windows checkouts;
- the style template inserted in Obsidian: no heading in its body;
- `frame` with integer vs decimal `size_m`, `origin_m` without `parent`, and an extra key in
  `frame`;
- pin lines with both `feature` and `r`, with neither, `0`, `1`, `1.0`, `0.5,0.5` without a
  space, `·` vs `-` separators, `|shown text` links;
- `on::` with `→` vs `->`;
- step attribute lines after a blank line (not "directly after"); step numbers with gaps;
  `after::` naming a step that has no arrow into the join step; loops with and without
  `repeat::`;
- example ids at the edges of the reserved block (`Q-9900`, `Q-9901`, `Q-9999`, `Q-10000`);
- guide fences without a path in the info string, and paths with `\`;
- a guide whose first block is not a paragraph.

## 9. Left to the builder

- Exact wording of READMEs, guide prose, template body lines and snippet descriptions.
- The exact synthetic records of the worked examples, within §4.6.
- File layout inside `tools/repo_checks/` and the pinned `markdown-it-py` version.
- Whether `markdown-it-py` is a runtime or a dev dependency of the `repo-checks` project.
