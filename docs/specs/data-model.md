# Content Data Model — schema v1

Status: accepted by Jann on 31 Oct 2026 (with the generative additions in §8 and §10).
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
| truth, belief, clue, script | `T-`, `B-`, `C-`, `S-` + at least 3 digits | `T-012`, `S-004` |
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
| `status` | `draft` \| `canon` | lore kinds, floor, script | Canon records are locked against agents. There is no "proposed" status: proposals live on agent branches. |
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
| topic | `lore/topics/` | Codex | explaining counterpart (in-world only) |
| truth | `lore/truths/` | Codex | required truths, topic, equations |
| belief | `lore/beliefs/` | Codex | holders, distorted truth, topic |
| clue | `lore/clues/` | Codex | revealed truth, where, earlier clues |
| equation | `canon/formalism/` | Codex (generated, read-only) | – |
| script | `story/scripts/` | Scripts | where; beats link places, speakers, beliefs, clues |
| dialogue | `dialogue/` | Scripts | speaker |
| template | `templates/` | Scripts | – |
| concept | `concepts/` | Concepts | what it is for, picked versions |
| concept-version | `concepts/<concept>/` | Concepts | concept, source version |
| map | `maps/` | Maps | what it is for, picked versions |
| map-version | `maps/<map>/` | Maps | map, source version |

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
| location | `type` | text | no | shrine, choir-hall, echo-vault, settlement, … |

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
| `type` | text | yes | artifact, document, creature, … |
| `where` | list of ids → band, floor, location | no | Where it can be found. |

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
| dialogue | `speaker` | id → npc, faction, people | yes | Kept lines are voice blocks in the body; generation details go in the commit trailer. |
| template | `purpose` | `dialogue` \| `description` | yes | The body is the prompt with `{{placeholders}}`. |

## 8. Generative records

Concepts and maps share one model, so any generator — ComfyUI today, another map renderer or the
game engine later — fits without new fields. Nothing in this section names a specific generator.

**Parents** (`concept`, `map`) hold the brief in the body and the notes (§9.3).

| Kind | Field | Type | Req. | Meaning |
|---|---|---|---|---|
| concept | `for` | list of ids → any world, people or thing record | yes | |
| concept | `picked` | list of ids → concept-version | no | Canon version(s); one per state for two-state concepts. |
| map | `for` | list of ids → band, floor, location | yes | |
| map | `type` | text | no | Free text: terrain, settlement, interior, … |
| map | `picked` | list of ids → map-version | no | Canon version(s). |

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

The editor never interprets an output file. A map's `package` output is the engine's input
(theos-map package, Confluence Decision 221741058); its format belongs to the generator and the
engine, not to this schema.

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
ledger, the index, search data, embeddings and compile previews (all in `.theos/cache`).

## 12. Links: stored once

| Stored on | Field | Derived on the other side |
|---|---|---|
| child | `parent` | children |
| belief | `held_by`, `distorts` | what a holder believes; how a truth is held |
| clue | `reveals`, `where` | how a truth is revealed; the clues of a floor; the ledger |
| truth, belief | `topic` | truths and beliefs of a topic |
| in-world topic | `counterpart` | the explaining topic's in-world page |
| script beat | `at`, `with`, `says`, `reveals` | where a record appears in the story; the cast |
| concept, map | `for` | concepts and maps of a record |
| version | parent, `from`, `addresses` | versions of a parent; lineage; which versions answer a note |
| any body | `[[id]]` | mentions |

## 13. theos.toml

```toml
[project]
name           = "tower-of-theos"
kind           = "content"
schema_version = 1

[parts]
enabled = ["codex", "changes", "scripts", "graph", "concepts", "maps", "review", "mcp"]
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
