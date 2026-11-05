# Content Data Model — schema v1

Status: accepted by Jann on 31 Oct 2026 (with the generative additions in §8 and §10). World and
art additions (enemy, spawn, quest, style; pins, cameras and frames on maps; concept types) accepted
with umbrella spec version 2. All additions are additive, so the schema stays version 1.
Readable copy for people: Confluence TOT › "Content Data Model (schema v1)". This file is binding.

The contract between the content files, `theos-content`, the CLI, the MCP server and the editor UI.

## 1. Background

- The 12 Confluence lore pages (6 topic pairs: The Tower & the Weave, The Summit & the Endings,
  Runes, Chanting, Martial Arts, Spirit Magic; each an Explaining and an In-World page) hold prose
  only. No truth, belief or clue IDs exist yet. The import turns the 12 pages into 12 `topic`
  records; truths, beliefs and clues are extracted afterwards as proposals.
- Shared Formalism cites equations as `E1`…`E18`; the read-only mirror uses exactly those ids.

## 2. Files and identity

- One record per file. UTF-8, LF, YAML front matter between `---` lines, then a Markdown body.
- The file name is the id (`T-012.md`, `f04-choir-hall.md`). Ids are unique across the project.
- World and people trees: children sit in a folder next to their parent
  (`world/hearth.md` and `world/hearth/f04-choir-hall.md`). The parent's path never changes when
  children are added. Generated versions sit in a folder next to their concept or map. All other
  kinds sit flat in their kind folder.
- Links in prose are wikilinks: `[[id]]` or `[[id|shown text]]`. Links in front matter are plain ids.
- Git history, author and dates are not stored in records.

## 3. IDs

| Kinds | Pattern | Example |
|---|---|---|
| truth, belief, clue, script, quest | `T-`, `B-`, `C-`, `S-`, `Q-` + at least 3 digits | `T-012`, `S-004`, `Q-012` |
| equation (mirror) | `E` + number, as in Shared Formalism | `E7` |
| floor | slug starting with `f` + two digits | `f04-choir-hall` |
| concept-version, map-version | parent id + `-v` + number | `fates-shrine-v5`, `f04-terrain-v3` |
| topic | slug ending in `-explaining` or `-in-world` | `spirit-magic-in-world` |
| everything else | slug: `^[a-z0-9]+(-[a-z0-9]+)*$` | `mother-ilse` |

Numbers are never reused. An agent session reserves a block of numbers per prefix when it starts.

## 4. Restricted YAML

Allowed:
- keys `^[a-z][a-z0-9_]*$`, each once;
- scalars: plain, single- or double-quoted strings, integers, decimals, `true` / `false`;
- lists of scalars, either one-line `[a, b]` or `- item` lines;
- one level of map whose values are scalars or one-line lists; map keys follow the key rule;
- comments on their own line or after a value.

Not allowed (the check reports line and column): anchors, aliases, tags (`&`, `*`, `!`);
multi-line scalars (`|`, `>`); flow maps `{…}`; lists of maps; maps inside maps; tab indentation;
duplicate keys.

Editing: a span patcher rewrites only the changed field's lines. Comments, blank lines and key
order survive. A new field is inserted after the last present field that precedes it in the kind's
field order (the order of the tables below).

## 5. Fields every record has

| Field | Type | Required | Meaning |
|---|---|---|---|
| `id` | id | yes | Identity; equals the file name. |
| `kind` | kind name | yes | One of §6. |
| `title` | text | yes | Name or one-line statement. |
| `status` | `draft` \| `canon` | lore kinds, floor, script, quest | Canon records are locked against agents. There is no "proposed" status: proposals live on agent branches. |
| `tags` | list of text | no | Free tags. |

## 6. Kinds

| Kind | Folder | Editor part | Stores links to |
|---|---|---|---|
| tower | `world/tower.md` | Codex | – |
| band | `world/` | Codex | parent tower |
| floor | `world/<band>/` | Codex | parent band |
| location | under its floor or location | Codex | parent |
| people | `people/` | Codex | bands it lives in |
| faction | `people/<people>/` | Codex | parent people |
| npc | under its faction or people | Codex | parent, home |
| thing | `things/` | Codex | where it is found |
| enemy | `enemies/` | Codex | bands it lives in |
| spawn | under its floor or location | Codex, Atlas | parent, what appears |
| topic | `lore/topics/` | Codex | explaining counterpart (in-world only) |
| truth | `lore/truths/` | Codex | required truths, topic, equations |
| belief | `lore/beliefs/` | Codex | holders, distorted truth, topic |
| clue | `lore/clues/` | Codex | revealed truth, where, earlier clues |
| equation | `canon/formalism/` | Codex (generated, read-only) | – |
| script | `story/scripts/` | Scripts | where; beats link places, speakers, beliefs, clues |
| quest | `story/quests/` | Scripts | giver, where, required quests; steps link places, people, spawns, dialogue, scripts, clues, quests |
| dialogue | `dialogue/` | Scripts | speaker |
| template | `templates/` | Scripts | – |
| concept | `concepts/` | Concepts | what it is for, picked versions |
| concept-version | `concepts/<concept>/` | Concepts | concept, source version |
| map | `maps/` | Maps, Atlas | what it is for, picked versions; pins and cameras in the body |
| map-version | `maps/<map>/` | Maps | map, source version, parent version of its frame |
| style | `art/` | Concepts | what it covers, the style it extends |

## 7. Fields per kind

### World

| Kind | Field | Type | Req. | Meaning |
|---|---|---|---|---|
| band | `parent` | id → tower | yes | |
| band | `floors` | list of 2 integers | yes | First and last floor, e.g. `[1, 20]`. |
| band | `mode`, `mood`, `belief_stage` | text | no | As in the canon band table. |
| band | `tier` | `T1` \| `T2` \| `T3` | no | Typical Understanding tier. |
| floor | `parent` | id → band | yes | |
| floor | `number` | 1–100 | yes | Inside the band's range. |
| location | `parent` | id → floor, location | yes | |
| location | `type` | text | no | shrine, choir-hall, echo-vault, settlement, portal, … |
| spawn | `parent` | id → floor, location | yes | |
| spawn | `of` | list of ids → enemy, faction, people, npc | yes | What appears here: enemies, or hostile people such as bandits or a boss. |
| spawn | `threat` | integer 1–5 | no | Threat level at design depth. |
| spawn | `density` | `low` \| `medium` \| `high` | no | |

Where a spawn is on a map is a pin in that map's body (§9.6), not a field of the spawn. A spawn
that no quest step names in `fights::` is ambient.

### People and lenses

| Kind | Field | Type | Req. | Meaning |
|---|---|---|---|---|
| people | `lives_in` | list of ids → band, floor | no | |
| faction | `parent` | id → people | yes | |
| npc | `parent` | id → faction, people | yes | |
| npc | `home` | id → location, floor | no | |
| npc | `role` | text | no | |
| people, faction, npc | `lens` | map | no | See below. A child inherits each lens field it does not set. |

| Lens field | Type | Meaning |
|---|---|---|
| `thinks_from` | text | The frame everything is explained through. |
| `stance` | text | Attitude in a few words. |
| `tier` | T1 \| T2 \| T3 | The Understanding this holder speaks from. |
| `unaware_of` | list of ids → truth | Truths this holder cannot know; never sent to an LLM in their voice. |
| `rejects` | list of ids → belief | Beliefs of others this holder argues against. |
| `speaks_to` | text | Who they talk to and how. |

What a holder believes is derived from beliefs whose `held_by` names it or a parent.

### Things

| Field | Type | Req. | Meaning |
|---|---|---|---|
| `type` | text | yes | artifact, document, creature, clothing, … |
| `where` | list of ids → band, floor, location | no | Where it can be found. |

### Enemies

| Field | Type | Req. | Meaning |
|---|---|---|---|
| `bands` | list of ids → band | no | Where it lives. |
| `threat` | integer 1–5 | no | Threat level at design depth. |
| `role` | `swarm` \| `brute` \| `ranged` \| `elite` \| `boss` | no | |

The body says what the enemy is and what the folk believe it is. Tuning numbers (stats, spawn
tables, timers, loot) are not part of this schema; the engine part adds them later as fields of
its own.

### Lore

| Kind | Field | Type | Req. | Meaning |
|---|---|---|---|---|
| topic | `track` | `explaining` \| `in-world` | yes | The voice of the body. |
| topic | `counterpart` | id → topic | in-world only | The explaining topic it converges on. |
| topic | `source` | URL | no | The frozen Confluence page it came from. |
| truth | `target` | `hinted` \| `suspected` \| `confirmed` \| `never` | yes | How far the player may get. `never` truths are locked and withheld from LLM prompts. |
| truth | `requires` | list of ids → truth | no | Truths that must be confirmed first. |
| truth | `topic` | id → topic (explaining) | no | |
| truth | `cites` | list of ids → equation | no | |
| belief | `held_by` | list of ids → people, faction, npc | yes | |
| belief | `distorts` | id → truth | no | |
| belief | `accuracy` | `false` \| `partial` \| `close` | no | Nearness to the distorted truth. |
| belief | `topic` | id → topic (in-world) | no | |
| clue | `reveals` | id → truth | yes | |
| clue | `moves_to` | `hinted` \| `suspected` \| `confirmed` | yes | State reached when found. |
| clue | `where` | list of ids → band, floor, location | yes | A band counts as its first floor for the ledger. |
| clue | `gate` | T1 \| T2 \| T3 | yes | Understanding needed. |
| clue | `after` | list of ids → clue | no | Clues needed first. |
| clue | `vector` | text | yes | The in-game carrier, designer voice. |
| equation | `source` | URL | yes | Anchor on Shared Formalism. |
| equation | `generated` | `true` | yes | A hand edit fails the check. |

Truth and clue titles and bodies are developer voice. Belief titles are in-world voice.

### Story

| Kind | Field | Type | Req. | Meaning |
|---|---|---|---|---|
| script | `where` | list of ids → band, floor, location | no | |
| quest | `type` | `main` \| `trial` \| `side` | yes | Main: why the player climbs, across floors. Trial: clears one floor and opens the way up. Side: helps the folk of the Tower. |
| quest | `giver` | id → npc, faction, people | no | |
| quest | `where` | list of ids → band, floor, location | yes | Every step's `at` lies inside. |
| quest | `requires` | list of ids → quest | no | Quests that must be completed before this one is offered. |
| dialogue | `speaker` | id → npc, faction, people | yes | Kept lines are voice blocks in the body; generation details go in the commit trailer. A dialogue used by a quest step with outcomes has one level-2 section per outcome (§9.5). |
| template | `purpose` | `dialogue` \| `description` \| `prompt` | yes | The body is the prompt with `{{placeholders}}`. `prompt` templates build briefs for generators (§9.9). |

### Art

| Kind | Field | Type | Req. | Meaning |
|---|---|---|---|---|
| style | `for` | list of ids → tower, band, people, faction | yes | What the style covers. |
| style | `extends` | id → style | no | A band style extends the tower style; a people or faction style extends a band style. |
| style | `state` | `draft` \| `locked` | yes | A change to a locked style is high significance in review. Locking and unlocking are never done over MCP. |
| style | `refs` | map: role → binary ref | no | Approved crops used as style references, e.g. `buildings`, `sky`, `people`. |

The effective style of a concept resolves like a lens, from the records in its `for`: the style of
the faction or people a record is or belongs to, then the style of the band the concept's place
sits in, then the tower style. Each body section comes from the nearest style that has it, and the
editor shows where each one came from. The body sections are listed in §9.8.

## 8. Generative records

Concepts and maps share one model, so any generator — ComfyUI today, another map renderer or the
game engine later — fits without new fields. Nothing in this section names a specific generator.

**Parents** (`concept`, `map`) hold the brief in the body and the notes (§9.3).

| Kind | Field | Type | Req. | Meaning |
|---|---|---|---|---|
| concept | `for` | list of ids → any world, people or thing record | yes | |
| concept | `picked` | list of ids → concept-version | no | Canon version(s); one per state for two-state concepts. |
| concept | `type` | text | no | Free text naming the asset: key-art, vista, character-sheet, portrait, in-scene, creature-sheet, object-sheet, icon, inworld-map, … A pipeline sets it (`docs/generators.md` §8). |
| map | `for` | list of ids → band, floor, location | yes | |
| map | `type` | text | no | Free text: terrain, settlement, interior, blockout, … |
| map | `picked` | list of ids → map-version | no | Canon version(s). |

A map's body may hold `## Pins` (§9.6) and `## Cameras` (§9.7) next to the brief and the notes.

**Versions** (`concept-version`, `map-version`) are runs worth keeping.

| Field | Type | Req. | Meaning |
|---|---|---|---|
| `concept` / `map` | id → parent | yes | |
| `generator` | `name@version` | yes | As reported by the generator's `describe`. |
| `recipe` | text | yes | Recipe, workflow or template id known to the generator. |
| `seed` | integer | no | When the generator uses one. |
| `inputs` | map | no | Open: whatever the recipe takes (model, steps, strength, prompt pieces, …). |
| `from` | id → version of the same kind | no | Source version (image-to-image, refinement, re-run). |
| `outputs` | map: role → binary ref | yes | Open roles, e.g. `image`, `truth`, `inworld`, `heightfield`, `package`. |
| `results` | map | no | Numbers and flags the generator reports (constraint values, pass). |
| `addresses` | list of note ids | no | Notes of the parent this version answers. |
| `rating` | `unrated` \| `keep` \| `kill` \| `mix` | yes | |
| `state` | T1 \| T3 | no | Concept versions of two-state concepts. |
| `frame` | map | no | Map versions only. The map's frame, written by the editor from the run's manifest (`docs/generators.md` §3): `size_m` (two numbers, width and height in metres); for a child map also `parent` (id → the map-version it sits in) and `origin_m` (two numbers, its north-west corner in the parent's frame). |

Runs of a pipeline record `pipeline` and `stage` in `inputs` (`docs/generators.md` §8).

The editor interprets one output role and no other: `features` of a map version, GeoJSON in the
map's frame (x east, y south, metres, origin north-west; `docs/generators.md` §3). It reads it to
show generated features on the atlas and to bind pins to them. Every other output, among them a
map's `package` (the engine's input; theos-map package, Confluence Decision 221741058), belongs to
the generator and the engine, not to this schema.

A version without a `frame` takes no pins.

## 9. Body grammar

### 9.1 Voice blocks

In-world speech is an Obsidian callout of type `voice`. The title holds the speaker, optionally
the beliefs voiced and the tier; a last line starting with `— ` is the caption.

```text
> [!voice] [[throat-kin]] · [[B-020]] · T2
> Sing a name the way it was first sung and they lean close to hear it.
> — a binder, teaching the naming-song
```

### 9.2 Design notes

Inside in-world text, designer remarks go in a `> [!design]` callout or on a line starting with
`Design note:`. The in-world word check skips both.

### 9.3 Notes on concepts and maps

Notes collect feedback for the next generation. They are callouts of type `note` in the body of a
concept or map record, by convention under `## Notes`. The title holds a note id local to the
record (`n1`, `n2`, …), optionally the version it is about, and its state.

```text
## Notes

> [!note] n3 · [[f04-terrain-v2]] · open
> Lakes cover a tenth of the floor; the valley should drain east, toward the rim.
> — Jann

> [!note] n4 · resolved by [[fates-shrine-v5]]
> The roof reads as a church; make it a shelter built onto machinery.
```

- States: `open`, `resolved by [[<version id>]]`, `dropped`.
- A version lists the notes it answers in `addresses`. Resolving a note is an explicit edit.
- Open notes are passed to generators in every run request (see `docs/generators.md`).

### 9.4 Beats

A script body is prose with beats. A beat is a level-2 heading `## Beat <n> — <name>`, followed
directly by attribute lines `key:: value`, then the beat's prose until the next level-2 heading.
A beat is referred to as `S-004#3`.

| Attribute | Value | Checked |
|---|---|---|
| `at::` | one link → location, floor | inside the script's `where` |
| `with::` | links → npc, faction, people | |
| `says::` | links → belief | the speaker's lens must not reject it |
| `reveals::` | links → clue | clue `where` contains `at`; clue gate ≤ `when`; never-confirm and no-overshoot |
| `when::` | T1 \| T2 \| T3 | |

In-world text for the word check: voice blocks everywhere, bodies of in-world topics, belief
titles, dialogue records.

### 9.5 Quest steps

A quest body is prose with steps, written like beats. A step is a level-2 heading
`## Step <n> — <name>`, followed directly by attribute lines `key:: value`, then the step's prose
until the next level-2 heading. A step is referred to as `Q-012#2`. Step 1 is where the quest
starts.

| Attribute | Value | Checked |
|---|---|---|
| `at::` | one link → location, floor | inside the quest's `where`; a step may have no place |
| `with::` | links → npc, faction, people | |
| `do::` | text | free text: find, talk, fight, bring, escort, defend, extract, … |
| `fights::` | links → spawn, enemy | |
| `dialogue::` | links → dialogue | the dialogue's speaker is in `with::` or is the quest's `giver` |
| `script::` | links → script beat (`[[S-004#3]]`) | the beat exists |
| `reveals::` | links → clue | as for beats (§9.4), on every path |
| `when::` | T1 \| T2 \| T3 | |
| `next::` | step numbers, comma-separated | the steps exist; several open at once |
| `on::` | `<outcome> → <step>` | one line per outcome; the step exists |
| `after::` | step numbers, comma-separated | the step opens only when all of them are done |
| `quest::` | one link → quest | the step completes when that quest does (a trial inside the main quest) |
| `ends::` | `success` \| `failure` | a path ends here |
| `repeat::` | `true` | marks a loop as intended |

Rules:
- Several arrows into one step mean any of them opens it; `after::` waits for all.
- Every step is reachable from step 1, every path reaches an `ends::`, and a loop needs
  `repeat:: true` on one of its steps. A chain of quests that contain each other through
  `quest::` is an error.
- The lore rules never-confirm, no-overshoot and reveal-order run on every path from step 1 to an
  ending, not on the quest as a whole.
- A step with `on::` outcomes and a `dialogue::` needs, in each linked dialogue, a level-2 section
  named after every outcome (`## pay`, `## refuse`) holding the lines for it.

### 9.6 Pins on maps

Records are placed on a map by lines in the map's body, under `## Pins`. The map holds the layout;
a record never stores its own position. One record can appear on many maps.

```text
## Pins
placed_on:: [[f04-terrain-v1]]

- [[thornstead]] · 0.759, 0.334 · feature city-1
- [[elder-vass]] · 0.455, 0.610
- [[reed-marsh-lurkers]] · 0.120, 0.840 · r 120 m
```

- `placed_on::` names the map version the pins were placed on.
- Each line: a link to a location, npc, spawn or thing; `x, y` as fractions of the frame (x east,
  y south, origin north-west, each from 0 to 1); optionally `feature <id>` (a feature of that
  version's `features` output) and `r <n> m` (the pin is an area with that radius).
- A record appears at most once on a map. A location or spawn must belong to the map's `for`
  place or to a place below it.
- Pin once, on the most detailed map. Pins of a map whose picked version has a `frame.parent`
  also show on the parent's map, at the position the frame gives; that projection is derived.
- When `placed_on` is not the picked version, the map has one warning. Re-binding moves pins with a
  `feature` to the nearest feature of the same type in the picked version, keeps the others and
  marks them for checking, and sets `placed_on`, in one command.

### 9.7 Cameras on maps

Views of a place for concept art are lines under `## Cameras` in the body of the map they look at.

```text
## Cameras
- gate-a6 · 0.62, 0.71 · h 62 m · look 0.55, 0.64 · h 115 m · fov 66 · portrait
```

- A camera id is a slug unique on its map and is referred to as `<map id>#<camera id>`.
- Fields: position `x, y` as fractions of the frame; `h` the height in metres above the map's base
  level; `look` the target position and its height; `fov` the vertical field of view in degrees;
  `portrait` for a tall image (wide is the default).
- A blockout run (`docs/generators.md` §3) renders each camera of its map; pipelines take those
  renders as shape controls.

### 9.8 Style bodies

A style body has level-2 sections that briefs draw from: `## Prompt`, `## Negative`, `## Palette`,
`## Light`, `## Materials`, and optionally one section per concept type (`## portrait`,
`## icon`, …). A section a style leaves out comes from the style it extends.

### 9.9 Look sections and briefs

Locations, people, factions, npcs, things and enemies may have a `## Look` section in designer
voice: what a picture of the record must show. A brief for a generator is built by a `template`
with `purpose: prompt` from the record and its `## Look`, its place chain, the people living there,
the effective style and the concept's open notes. A brief for a tier holds only what is revealed
by that tier, and text of a `never` truth is never in a brief.

## 10. Binary references

One scalar: the path relative to the project's binaries folder, `@sha256:`, and the hex hash.

```yaml
outputs:
  image: concepts/fates-shrine/v5.png@sha256:9d6239060afedc58dee92aab1a21add3…   # 64 hex digits
```

Paths may not contain `@`. A missing file or a different hash is a warning. The editor never
overwrites a binary it did not write.

## 11. Never stored, always derived

Backlinks, children, the cast of a script, what a holder believes, effective lenses, the reveal
ledger, the index, search data, embeddings and compile previews (all in `.theos/cache`). Also
derived: pins projected onto parent maps, the maps a record appears on, the paths of a quest,
effective styles and briefs.

## 12. Links: stored once

| Stored on | Field | Derived on the other side |
|---|---|---|
| child | `parent` | children |
| belief | `held_by`, `distorts` | what a holder believes; how a truth is held |
| clue | `reveals`, `where` | how a truth is revealed; the clues of a floor; the ledger |
| truth, belief | `topic` | truths and beliefs of a topic |
| in-world topic | `counterpart` | the explaining topic's in-world page |
| script beat | `at`, `with`, `says`, `reveals` | where a record appears in the story; the cast |
| spawn | `parent`, `of` | spawns of a place; where an enemy or a faction appears |
| quest | `giver`, `where`, `requires` | quests of a place or a person; what a quest unlocks |
| quest step | `at`, `with`, `fights`, `dialogue`, `script`, `reveals`, `quest` | quests through a place; where a dialogue, spawn or beat is used |
| map body | pins, cameras | maps a record appears on; where it is |
| style | `for`, `extends` | the styles of a record; effective styles |
| concept, map | `for` | concepts and maps of a record |
| version | parent, `from`, `addresses`, `frame.parent` | versions of a parent; lineage; which versions answer a note; child maps |
| any body | `[[id]]` | mentions |

## 13. theos.toml

```toml
[project]
name           = "tower-of-theos"
kind           = "content"
schema_version = 1

[parts]
enabled = ["codex", "atlas", "changes", "scripts", "graph", "concepts", "maps", "review", "mcp"]
```

Generators are registered in `.theos/settings.toml` (see `docs/generators.md`). The binaries
folder is set per machine in the user settings, never in the repo.

## 14. Examples

Ids and texts are illustrations; real ones come from the import.

```markdown
--- world/hearth/f04-choir-hall.md
---
id: f04-choir-hall
kind: floor
title: The Choir-Hall
parent: hearth
number: 4
status: draft
---
A half-buried resonating chamber the Hearth folk have turned into their holiest place.
```

```markdown
--- people/hearth-folk/fates-cult/mother-ilse.md
---
id: mother-ilse
kind: npc
title: Mother Ilse
parent: fates-cult
home: fates-shrine
role: keeper of the Fates' shrine
lens:
  speaks_to: children and newcomers; calls the player "stair-child"
---
```

```markdown
--- lore/clues/C-031.md
---
id: C-031
kind: clue
title: A spirit murmurs a word in a dead tongue
status: draft
reveals: T-012
moves_to: hinted
where: [fates-shrine]
gate: T1
vector: spirit murmur under the shrine, in the Tuners' language
---
```

```markdown
--- story/scripts/S-004.md
---
id: S-004
kind: script
title: The shrine hums
status: draft
where: [f04-choir-hall]
---
The player's first night under the Fates' shrine.

## Beat 1 — Arrival
at:: [[fates-shrine]]
with:: [[mother-ilse]]
says:: [[B-016]]
when:: T1

> [!voice] [[mother-ilse]] · [[B-016]] · T1
> The Fates spin for those who climb. Hum with me, stair-child, and they will hear you.

## Beat 2 — The murmur
at:: [[fates-shrine]]
reveals:: [[C-031]]
when:: T1
```

```markdown
--- maps/f04-terrain.md
---
id: f04-terrain
kind: map
title: Choir-Hall terrain
for: [f04-choir-hall]
type: terrain
picked: [f04-terrain-v1]
---
A grassland valley around a half-buried choir-hall; one extraction run crosses it.

## Notes

> [!note] n1 · [[f04-terrain-v2]] · open
> Too much relief for the Hearth; keep the floor walkable.

--- maps/f04-terrain/f04-terrain-v1.md
---
id: f04-terrain-v1
kind: map-version
title: Choir-Hall terrain, seed 1
map: f04-terrain
generator: theos_maps@0.3.0
recipe: hearth-valley
seed: 1
outputs:
  truth: maps/f04/s1-truth.png@sha256:…
  inworld: maps/f04/s1-inworld.png@sha256:…
  package: maps/f04/s1_engine.zip@sha256:…
results:
  relief_m: 1234
  lakes_pct: 2.99
  pass: true
rating: keep
---
```

```markdown
--- concepts/fates-shrine/fates-shrine-v5.md
---
id: fates-shrine-v5
kind: concept-version
title: Fates' shrine, finished from v1
concept: fates-shrine
generator: comfyui@0.4
recipe: finish-i2i
seed: 1101
from: fates-shrine-v1
inputs:
  model: z-image-turbo
  steps: 8
  strength: 0.4
outputs:
  image: concepts/fates-shrine/v5.png@sha256:…
addresses: [n4]
rating: unrated
---
```

The examples below show the world and art additions; their names are synthetic.

```markdown
--- enemies/marsh-lurker.md
---
id: marsh-lurker
kind: enemy
title: Marsh lurker
bands: [hearth]
threat: 2
role: swarm
---
What it is, and what the Hearth folk believe it is.

## Look
Low, reed-coloured, always in groups; eyes catch lantern light.

--- world/hearth/f04-choir-hall/reed-marsh/reed-marsh-lurkers.md
---
id: reed-marsh-lurkers
kind: spawn
title: Lurkers in the reed marsh
parent: reed-marsh
of: [marsh-lurker]
threat: 2
density: medium
---
```

```markdown
--- story/quests/Q-012.md
---
id: Q-012
kind: quest
title: The binders' price
status: draft
type: side
giver: elder-vass
where: [f04-choir-hall]
requires: [Q-010]
---
Why the folk need help, in designer voice.

## Step 1 — Find the lost choir-boy
at:: [[reed-marsh]]
do:: find
fights:: [[reed-marsh-lurkers]]
next:: 2

## Step 2 — Bring him home
at:: [[thornstead]]
with:: [[elder-vass]]
dialogue:: [[vass-thanks]]
on:: pay → 3
on:: refuse → 4

## Step 3 — The naming-song
at:: [[fates-shrine]]
reveals:: [[C-031]]
when:: T1
ends:: success

## Step 4 — Turned away
ends:: failure
```

```markdown
--- maps/f04-terrain.md (body, after the brief)
## Pins
placed_on:: [[f04-terrain-v1]]

- [[thornstead]] · 0.759, 0.334 · feature city-1
- [[fates-shrine]] · 0.850, 0.680 · feature shrine-1
- [[reed-marsh-lurkers]] · 0.250, 0.450 · r 600 m

## Cameras
- shrine-stair · 0.840, 0.700 · h 30 m · look 0.850, 0.680 · h 40 m · fov 50
```

```markdown
--- art/hearth-style.md
---
id: hearth-style
kind: style
title: Hearth
for: [hearth]
extends: tower-style
state: locked
refs:
  buildings: art/hearth/buildings.png@sha256:…
  sky: art/hearth/sky.png@sha256:…
---
## Prompt
…

## Palette
…
```
