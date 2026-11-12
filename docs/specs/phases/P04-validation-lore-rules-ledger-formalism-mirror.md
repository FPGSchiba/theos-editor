# P04 · Validation, lore rules, ledger, formalism mirror — phase spec

Status: waiting for Jann's approval · 10 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P04. Scope and acceptance criteria come from there; this
spec decides how.

## 1. Goal

After P04, `theos-content` judges a content project the way the canon needs:
- the checks that compare records: a floor inside its band, versions of the same parent, notes
  that exist, parent cycles, and the beat checks of data model §9.4;
- the lore rules of architecture §10: the guardrails never-confirm, no-overshoot and
  inworld-vocab, and the order rules reveal-order, requires and three-clue;
- P02's body data and malformed constructs, which P02 §5.12 and P03 §3 left to P04;
- configured severities from the project's `.theos/settings.toml`; nothing lowers a guardrail
  (G8);
- the reveal ledger, built from clue records into the cache and printed by `theos ledger`;
- binary references, checked against the binaries folder;
- the read-only mirror of Shared Formalism, generated from `canon/formalism/_source.json`.

A change validates the records it affects (CE-14.3). For each record, the index stores what its
checks across records read. A change re-runs exactly the records that read something the change
touched. A cold build runs every record. Both use the same code, so a history of changes and a
cold build leave the same cache.

Later phases build on it:
- P05 prints the findings and the ledger with `--json`, checks the staged snapshot in the hook,
  and serves `ledger_check`;
- P06 rejects a command whose preview has blocking findings, the neighbours' included (CE-20.1);
- P07 judges proposals the same way, and locks the mirror and `never` truths against agents;
- P14 shows the order-rule setting, and guardrails without an Off (CE-39.2);
- P15 shows never-confirm on the beat (CE-41.2) and the ledger on the truth screen;
- P17 runs every lore rule on a draft (CE-46.3);
- P21 adds quest steps as places where clues are revealed, and checks every path through a
  quest.

## 2. Acceptance criteria (verbatim from the umbrella)

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

## 3. Scope

**Goals**
- In `theos-content`:
  - the rule model: every rule has an id, a default severity and a class (guardrail,
    configurable, fixed), and the editor parts register theirs (architecture §6);
  - body rules in the record step: P02's malformed constructs become findings where the kind's
    `BodyGrammar` gives them meaning, and P02's well-formed body data is judged;
  - the lore step: checks across records with stored reads, its own tables, and the neighbours
    in `Update` and `Preview`;
  - the value checks across records that P03 §3 lists, the beat checks of data model §9.4, and
    the six lore rules;
  - the ledger, in the cache;
  - validation settings: a `[validate]` table in `.theos/settings.toml` with the order rules'
    severities, the project's developer words and the allowlist;
  - binary reference checks against a binaries folder that the caller names;
  - the formalism mirror: the format of `_source.json`, the generator and its checks.
- In `theos` (binary): `theos ledger` and `theos formalism`, with text output.
- Every part uses a crate where one exists (directive 2), all of them already in `Cargo.lock`:
  `toml` with `Spanned` for the settings, `regex` for developer words, `serde` and `serde_json`
  for `_source.json`, `sha2` for hashes, and `clap` and `anyhow` in `theos`. Our code is the
  rules, the stored reads and the ledger (§4.2, §5.1).

**Non-goals**
- Quest steps, quest paths and the quest kind (P21). In P04, "every path" means the climb, floor
  by floor (§5.7).
- `--json`, `theos check`, the hook and the GitHub Action (P05). Blocking a command (P06).
- Settings layers other than the built-in default and the project (§4.1). The settings screen
  (P14).
- The real `E1`…`E18` and the real list of developer words. Both wait for a private session
  (`docs/status.md`); P04 proves the mechanisms with synthetic fixtures.
- Locking records against agents (P07). Watching the binaries folder.
- Rows the UI references show that are no P04 rule: `two-state`, `open-decision`, `coverage`,
  `near-duplicate`, `compile`, the atlas and quest rows, and the lens screen's checks of a drafted
  line ("T-012 stays unconfirmed for a T1 player", P17's CE-46.3). They belong to the phases of
  those screens.

**Anti-goals** (P04 fails if any of these holds, even with every test green)
- A check across records whose result after a history of changes differs from a cold build.
- A setting, an API call or an allowlist entry that makes a guardrail finding a warning or makes
  it disappear. An allowlist entry exempts only the word it names, and only with a reason.
- A cold open of the CE-12.1 project over one second on the CI runners, or a one-file change
  that re-runs the checks of every record.
- A hand-written parser for TOML or JSON, or a hand-written word matcher (directive 2).
- Lore invented in fixtures, or a guess of what Shared Formalism or another Confluence page says.

## 4. Decisions

### 4.1 Decided by Jann (10 Nov 2026)

- **Ruling (Jann): checks across records store their reads.** Each record's checks across
  records read through a context that stores every id, every set of inbound links and every input
  file they read. A change re-runs exactly the records that read something it touched (§5.5).
  Jann chose this over hand-declared neighbours per rule and over a whole-project pass after every
  write.
- **Ruling (Jann): nothing reveals a `never` truth.** Architecture §10: "Nothing reveals a
  `never` truth." A clue whose `reveals` names one is a never-confirm error whatever its
  `moves_to`, and so is a beat that reveals such a clue (§5.8).
- **Ruling (Jann): developer words are built-in patterns plus a project list.** Built in is only
  what is never in-world text: record ids, tier tokens and rule names. Every other word is
  listed in `.theos/settings.toml`. If Confluence or the theos-design plugin already holds a
  list, a private session copies it there (§5.11, `docs/status.md`).
- **Ruling (Jann): validation settings come from the project file only.** The `[validate]`
  table of the tracked `.theos/settings.toml`, over the built-in default. There is no user
  layer, so the editor, the hook and CI give the same verdict on every machine. A bad entry is
  an error finding at the entry and has no effect (§5.12).

### 4.2 Probes (10 Nov 2026, toolchain 1.99.1, Windows 11)

A scratch crate outside the repo used the versions pinned in `Cargo.lock`.

- **`toml` 1.1.8 with `Spanned`.** A `[validate.severity]` table deserialised as a map of
  `Spanned<String>` keys and values gives the line and column of each key and each value. An
  array of inline tables gives the span of each `allow` entry. An entry without `reason` reads as
  `None`, at its entry's span. A parse error carries its line and column.
- **`regex` 1.13.1.** `(?i)\b(?:ledger|canon|never-confirm)\b` finds whole words in Unicode
  text:
  - `The Ledger of names` and `Mère's canon.` match;
  - `ledgers`, `canonical`, `xledger` and `a never-confirmed thing` do not;
  - `ledger’s` (U+2019) matches `ledger`.

  The built-in pattern `\b(?:[TBCSQ]-[0-9]{3,}|E[0-9]+|T[1-3]|f[0-9]{2}(?:-[a-z0-9]+)*)\b`
  finds `T-012`, `E7`, `E18`, `T2`, `f04` and `f04-choir-hall`, and not `T-01`, `T4` or `XE7`.
- **`serde_json` 1.0.154.** A syntax error carries its line and column (`trailing comma at line
  3 column 32`).
- `clap` 4.6.7, `anyhow`, `serde` 1.0.229 and `sha2` are already workspace dependencies. P04
  adds no crate to the lockfile.

### 4.3 Approaches considered

| Question | Chosen | Considered |
|---|---|---|
| Neighbours (CE-14.3) | stored reads per record (Jann) | each rule names, by hand, the records a change affects, like P03's relink (a missed case makes the cache differ from a cold build); a pass over the whole project after every write (cost grows with the project on every write and preview, and P03's reading of CE-12.2 allows only relinking as other work) |
| Incremental engine | our own, over the SQLite store | `salsa` and `comemo` keep their tables in memory inside one process; the index is a SQLite cache shared by the editor, `theos` and every `theos-mcp`, and it must survive restarts |
| A `never` truth | nothing reveals it (Jann) | hints and suspicion allowed, only a confirming clue an error (the rule's name; the script screen's "must stay unproven") |
| Developer words | built-in patterns plus a project list (Jann) | also a built-in list of the data model's terms (truth, belief, clue, ledger, canon, tier, gate, beat), which flags common English words in in-world text from day one |
| Settings layers | project file only (Jann) | part default → user → project (G14): a user's lowered order rule would make the hook and CI disagree with the editor |
| Settings errors | an error finding at the entry, and the entry has no effect | refusing the whole file (one typo would also drop the project's developer words) |
| Binary checks | on request, not stored | stored with the other findings (the binaries folder is per machine, so processes with different folders would keep rewriting the shared cache) |
| Rules in SQL | no: rules are Rust modules over the context | SQL views and recursive CTEs (word matching and rendering the mirror do not fit SQL, and a rule in a CTE is harder to read and test than a small module) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Module (indicative) | Crate or tool | Our code, and why |
|---|---|---|---|
| Rule model and registration | `rules` | – | Ids, classes and registration (§5.2) |
| Body rules | `check::body` | P02's body model | One module per rule, one list (§5.3) |
| Facts for the lore step | `index::rows` | P02's model | Notes, `when::` tokens, in-world segments, input files (§5.4) |
| Lore step and stored reads | `index::lore` | SQL over the store | Subjects, the context, dirty set (§5.5) |
| Places and floors | `index::lore::places` | – | Data model §7, a walk along `parent` (§5.6) |
| Ledger | `index::lore::ledger` | – | §5.7 |
| Lore, story and value rules | `parts::*` | – | One module per rule, one line in its part (§5.8–5.10) |
| Developer words | `lore::vocab` | `regex` | Patterns and words as data (§5.11) |
| Validation settings | `settings` | `toml` (`Spanned`), `serde` | The `[validate]` table and its findings (§5.12) |
| Binary references | `index::binaries` | `sha2`, `std::io::copy` | Stamps and a hash memo (§5.13) |
| Formalism mirror | `formalism` | `serde_json`, `serde`, P02's YAML encoder | Format, rendering, checks (§5.14) |
| Cycles | `index::lore::cycle` | – | A walk from one record along one field, storing each read; `petgraph` would need the whole graph built first, which defeats reading per record |
| CLI | `theos` | `clap` (derive), `anyhow` | `ledger` and `formalism` (§5.15) |

New dependencies of `theos-content`: `serde` (derive) and `serde_json` as normal dependencies
(today `serde` is a dev dependency only). `theos` gets `clap`, `anyhow` and `theos-content`.

### 5.2 Rules: ids, classes and registration

- Rule ids are `<group>/<name>` (P03 §9). P04's groups are `body/`, `world/`, `link/` (parent
  cycles), `story/`, `lore/`, `version/`, `binary/`, `formalism/` and `settings/`.
- **Classes:**

| Class | Rules | Severity |
|---|---|---|
| guardrail | `lore/never-confirm`, `lore/no-overshoot`, `lore/inworld-vocab` | error, always; no setting names them (G8) |
| configurable | `lore/reveal-order` (default error), `lore/requires` (default error), `lore/three-clue` (default warning) | error or warning, by setting (architecture §10) |
| fixed | every other rule, P02's and P03's included | its default |

- **Registration.** A rule that belongs to an editor part is registered by that part, through
  the `KindRegistry` and with the part's kinds (architecture §6): `register_rule(part, RuleSpec)`.
  Codex registers the world, lore, link and formalism rules; Scripts the story rules; Concepts
  and Maps the version rules. A `RuleSpec` names the rule's id, default severity and class, the
  kinds whose records it checks, and its function. **A new rule is one module plus one line in
  its part.**
- A rule may check records of any kind, its own part's or another's: never-confirm, a Codex
  rule, also checks scripts. One rule id may be registered by several parts, each for its own
  kinds, when the severity and the class agree. The version rules are registered by Concepts for
  concept versions and by Maps for map versions.
- The body rules of §5.3 judge P02's grammar for every kind, through `BodyGrammar`. They are a
  list in `check/`, like P03's `SCHEMA_RULES`. The settings rules (§5.12) belong to no part and
  are a list of their own.
- Every rule joins the rule set fingerprint (P03 §5.8) with its id, default severity and class,
  so a new rule rebuilds old caches.
- `rules()` lists every rule with its id, group, default severity and class. P05 prints it, and
  P14 builds the settings form from it.

### 5.3 The record step: body rules

These rules run in `check_record`, after P03's rules, and their findings have the step
`record`. Each reads the parsed body and the kind's `BodyGrammar`.

| Rule | Severity | When | Kinds (`BodyGrammar`) | Position |
|---|---|---|---|---|
| `body/link-unclosed`, `body/link-target`, `body/embed` | error | P02's malformed link forms (P02 §5.8) | every kind | the construct |
| `body/callout-title` | error | a malformed voice title, in every kind; a malformed note title, where notes mean something | voice: every kind; note: concept, map | the title |
| `body/beat-heading`, `body/beat-attribute`, `body/misplaced-attributes` | error | P02's malformed beat forms (P02 §5.10) | kinds with beats (script) | the construct |
| `body/tier` | error | a voice title's tier token is not `T1`, `T2` or `T3` | every kind | the token |
| `body/when` | error | a `when::` value is not `T1`, `T2` or `T3` | kinds with beats | the value |
| `body/attribute-count` | error | `at::` holds more than one link (data model §9.4: "one link") | kinds with beats | the second link |
| `body/attribute-duplicate` | error | a key appears twice in one beat | kinds with beats | the second key |
| `body/attribute-unknown` | warning | a key that the kind's beat attributes do not have | kinds with beats | the key |
| `body/beat-number` | error | two beats of one record have the same number | kinds with beats | the second heading |
| `body/note-id` | error | two notes of one record have the same id | kinds with notes | the second id |
| `body/note-state` | error | a note's state is none of `open`, `resolved by [[id]]`, `dropped` (CE-15.5) | kinds with notes | the state |

- Each of P02's `MalformedKind`s maps to one `body/` rule, by its name.
- In a kind whose grammar has no beats or notes, a beat heading is an ordinary heading and a
  `> [!note]` is an ordinary callout (P03 §5.4). They give no finding there.
- One fault gives one finding. A construct P02 reports as malformed gets only its `body/`
  finding.
- `BodyGrammar` gains the attributes that are not links (`when` for beats), so
  `body/attribute-unknown` reads one table. P21 adds the quest step attributes the same way.

### 5.4 What the record step stores for the lore step

The lore step reads only rows. The record step adds these to P03's rows (`records`, the
`field_values` with their `valid` flag, `links` with their slots and beat numbers, `beats`, and
the `files` hash):
- for each beat, its `when::` token and span;
- for each note (concept, map): its id, state, version link and spans;
- the record's in-world text as segments with their spans (§5.11);
- the **input files** `.theos/settings.toml` and `canon/formalism/_source.json`: path, SHA-256
  and text. Every sync stats both, like a record (the walk never enters `.theos/`, so the sync
  names that one). `apply` and `preview` take them as changes too, so P14 can preview a settings
  change.

### 5.5 The lore step: subjects, stored reads and neighbours (CE-14.3)

**Subjects.** Every record is a subject, and so is each input file. A subject's rules are those
registered for its kind (§5.2); `_source.json` has the formalism rules and
`.theos/settings.toml` the settings rules.

**The context.** A subject's rules run together through one context. It is the only way a rule
gets data: rules get no SQL connection. Every answer adds a key to the subject's reads:

| Key | Answer | Changes when |
|---|---|---|
| `rec:<id>` | the facts of every record with that id: none, one, or several when the id is duplicated | a record with that id is re-parsed, added or removed |
| `in:<id>` | the links that name the id: source id, path, slot, beat number, span | a record whose old or new rows link to the id is re-parsed, added or removed |
| `file:<path>` | an input file, parsed | the file is added, changed or removed |

A record's facts are its kind, path, title, status, file hash, valid field values, beats with
their attributes, notes and in-world segments. Helpers built on the context are recorded through
it like any rule: the floor of a place (§5.6), the ledger of a truth (§5.7), the effective
`rejects` of a holder (P03 §5.12's chain, one implementation for both), and cycles (§5.10).

**What a subject produces:** its findings (step `lore`), its ledger rows (truths), and its reads.
A finding of the lore step is always on its own subject. A finding on an input file has its
project-relative path and position, and no id.

**When it runs.** The lore step is step 4b of P03 §5.9, after the relink and the duplicate-id
recheck, in the same transaction:
1. The changed keys: `rec:<id>` for the old and the new id of each re-parsed, added or removed
   record; `in:<id>` for each id that its old or new link rows name; `file:<path>` for each
   changed input file.
2. The dirty subjects: the re-parsed and added records, the changed input files, and every
   subject whose stored reads hold a changed key. Removed subjects lose their rows.
3. Each dirty subject: delete its lore findings, ledger rows and reads; run its rules; store what
   they give.

There is no second round: rules read only rows of the record step and P03's links, never another
subject's results. When `requires` needs the ledger of another truth, it computes it in its own
run, and its reads include everything that ledger read.

**Cold build and incremental run use the same code.** A cold build runs every subject; its
context first loads all facts in bulk, so a cold build makes no query per read (CE-12.1). An
incremental run fetches facts by id when a rule asks, and keeps them for the rest of the run.

**Reads are coarse.** Any re-parse changes `rec:<id>`, also a prose edit. That is correct and
simple. The plan may narrow it, for example to a digest of the facts, only with the history test
of §5.17 still equal.

**Neighbours.** `Update` gains `revalidated`: the subjects the lore step re-ran that were neither
re-parsed nor relinked. `Preview.findings` includes their findings. These are CE-14.3's
neighbours, and P06 rejects a command on them too.

### 5.6 Places and floors

- The floor of a place:
  - a floor: its `number`;
  - a band: its first floor (data model §7: "A band counts as its first floor for the ledger");
  - a location: the floor of its parent, through any number of locations.
- A place is inside another when it is that place, or that place is on its parent chain (a
  location inside its floor and that floor's band).
- A chain that dangles, links to a wrong kind, meets an invalid value or loops ends without a
  floor. Rules that need the floor skip that place; the chain's own finding exists already
  (`link/dangling`, `link/kind`, `schema/value`, `link/parent-cycle`).

### 5.7 The ledger (CE-15.3)

**"On every path" in P04 is the climb.** The player meets floors in rising order and gains
Understanding T1, then T2, then T3. A clue's **availability** is its floor (the lowest floor
among its `where` places) and its `gate`. Clue A **can be found no later than** clue C when A's
floor is at most C's and A's gate is at most C's. Then on every climb that reaches C, A was
available first. A path through a quest's steps is P21's (CE-58.3).

**Rows.** For each truth, one row per clue whose `reveals` names it and whose `moves_to` is
valid: state (`moves_to`), floor (none when no place gives one), the place that gives it, gate,
and the clue's id and path. With each clue, the beats that reveal it: script id and beat number
("placed by").

- The rows are stored in a `ledger` table and belong to the truth's subject (§5.5).
- `Index::ledger()` gives every truth, by id: id, path, title, target, and its rows in the
  order hinted, suspected, confirmed, then by floor, then by clue id; and the beats that place
  them. `ledger_of(id)` gives one truth.
- The first row of each state is the summary the truth screen shows ("hinted F04 · T1 ·
  C-031").

### 5.8 Lore rules (CE-15.1)

States are ordered hinted < suspected < confirmed. The target `never` stands outside that
order. A clue reveals its truth up to its `moves_to`. A beat reveals the clues in its
`reveals::` (P21 adds quest steps).

| Rule | Class | Finding when | On |
|---|---|---|---|
| `lore/never-confirm` | guardrail | a clue's `reveals` names a truth whose target is `never`, whatever its `moves_to` | the clue's `reveals` value |
| | | a beat's `reveals::` names such a clue | the script, at that link |
| `lore/no-overshoot` | guardrail | a clue's `moves_to` lies above its truth's target (target `hinted` or `suspected`) | the clue's `moves_to` value |
| | | a beat's `reveals::` names such a clue | the script, at that link |
| `lore/inworld-vocab` | guardrail | a developer word in in-world text (§5.11) | the word |
| `lore/reveal-order` | configurable, error | (a) a clue in `after` cannot be found no later than this clue (§5.7) | the `after` item |
| | | (b) a clue moves its truth to a state, the truth has clues of a lower state, and no clue of the nearest such state can be found no later than it | the clue's `moves_to` value |
| | | (c) the clue is on a cycle of `after` | the `after` item that leads into the cycle |
| `lore/requires` | configurable, error | for each `Y` in a truth `X`'s `requires`: (a) `Y`'s target is not `confirmed`, so `Y` can never be confirmed | the `requires` item |
| | | (b) a clue brings `X` to its target, and no clue that confirms `Y` can be found no later than that clue | the `requires` item, naming the clue |
| | | (c) `X` is on a cycle of `requires` | the `requires` item that leads into the cycle |
| `lore/three-clue` | configurable, warning | a truth with target `hinted`, `suspected` or `confirmed` has fewer than three clues whose `moves_to` is its target | the truth's `target` value |

- **The order of states** (reveal-order (b)) compares each clue with the nearest lower state the
  truth has clues for. "Can be found no later" is transitive, so when every state passes, all
  lower states are in order. A truth hinted on F04, suspected on F50 and confirmed on F21 gets
  one finding, on the confirming clue. When a middle state fails, only that state's clue gets
  the finding.
- Messages say what to change, as the UI references do: "C-940 confirms T-941, whose target is
  `never`: nothing may reveal it. Let a belief voice it instead."; "Only one clue moves T-940 to
  confirmed. The three-clue rule asks for three."
- **Skips.** A rule needs valid values. A truth without a valid `target`, a clue without a valid
  `moves_to` or `gate`, and a `reveals` that does not resolve to a truth are skipped by the rules
  that need them; P03's findings cover them.
- **Duplicated ids.** A link to a duplicated truth id is judged against every record with that
  id. A guardrail fires if any of them breaks it.
- Draft and canon clues count alike.

### 5.9 Story rules: beats (data model §9.4)

| Rule | Severity | Finding when | On |
|---|---|---|---|
| `story/beat-at` | error | the beat's `at::` is not inside any place of the script's `where` (a script without `where` is not checked) | the `at::` link |
| `story/says-rejected` | error | the effective lens of every holder in the beat's `with::` rejects a belief in its `says::` (a beat without `with::` is not checked) | the `says::` link |
| `story/reveal-where` | error | a clue in `reveals::` whose `where` does not contain the beat's `at::` | the `reveals::` link |
| `story/reveal-gate` | error | the beat's `when::` is below the gate of a clue in `reveals::` | the `reveals::` link |

- never-confirm and no-overshoot on beats are in §5.8.
- A beat without `at::` or without `when::` skips the checks that need it. P04 does not make
  either required.
- "Inside" is §5.6's.

### 5.10 Value checks across records (P03 §3)

| Rule | Severity | Finding when | On |
|---|---|---|---|
| `world/floor-number` | error | a floor's `number` lies outside its band's `floors` (when the parent is a band with valid `floors`) | the `number` value |
| `version/picked` | error | an item of a concept's or map's `picked` names a version whose `concept` or `map` is another record | the item |
| `version/from` | error | a version's `from` names a version of another parent | the `from` value |
| `version/addresses` | error | an item of `addresses` names a note id that the parent's notes do not have (CE-15.5; when the parent resolves) | the item |
| `link/parent-cycle` | error | the record's `parent` chain leads back to it | the `parent` value |

- **Cycles** along a link field (`parent`, `requires`, `after`) are found by a walk from the
  subject that stores each record it reads. A record whose chain runs into a cycle it is not
  part of gets no finding; the records on the cycle carry it.
- P21 reuses the walk for quests that contain each other (CE-58.6), and P26 for `extends`.

### 5.11 In-world text and inworld-vocab (G8)

**In-world text** (data model §9.2 and the last paragraph of §9.4):
- every voice block's content and caption, in every kind, but not its title;
- the body of an in-world topic;
- a belief's `title`;
- the body of a dialogue record, except its level-2 headings (outcome names, data model §9.5).

Never in-world text:
- `> [!design]` callouts and `Design note:` lines, also inside a voice block (data model §9.2);
- voice titles, also inside an in-world topic or a dialogue: they hold the speaker, the
  beliefs and the tier, not speech.

A link's shown text is in-world text. A link without shown text shows its id, so the id is
checked. A link target is otherwise markup.

**Developer words:**
- **Built in** (Jann's ruling, §4.1), matched case-sensitively:
  - ids of the numbered kinds and floors: `T-`, `B-`, `C-`, `S-`, `Q-` with three or more
    digits, `E` with a number, and `f` with two digits and an optional slug part
    (data model §3);
  - the tier tokens `T1`, `T2`, `T3`;
  - the rule names `never-confirm`, `no-overshoot`, `inworld-vocab`, `reveal-order` and
    `three-clue`. `requires` is an ordinary English word and is not listed.
- **The project's words**: `words` in `[validate.inworld-vocab]` (§5.12), matched as whole words,
  case-insensitively, at Unicode word boundaries (`regex`, §4.2).
- **The allowlist**: an `allow` entry with a reason exempts its word everywhere: a listed word
  case-insensitively, a built-in match by its exact text (`T1`).

Each occurrence is one finding, at the word: "`ledger` is a developer word, which in-world text
cannot use."

The record step stores the in-world segments (§5.4). The lore step matches them against the
words it reads from `.theos/settings.toml` (`file:.theos/settings.toml`), so a new word re-runs
exactly the records with in-world text.

### 5.12 Validation settings (CE-14.1, CE-15.2)

P04 reads the `[validate]` table of `.theos/settings.toml` (tracked) over the built-in default.
Other tables of that file belong to other parts (P09's generators) and are not read. There is no
user layer (§4.1).

```toml
[validate.severity]
"lore/three-clue"   = "error"
"lore/reveal-order" = "warning"

[validate.inworld-vocab]
words = ["synthetic-word"]
allow = [
  { word = "T1", reason = "synthetic example of a reason" },
]
```

The file is an input of the index (§5.4). Its problems are findings on `.theos/settings.toml`,
with line and column from `toml`'s spans:

| Rule | Severity | Finding when | On | Effect |
|---|---|---|---|---|
| `settings/toml` | error | the file is not valid TOML, or `[validate]` does not have the shape above | `toml`'s position | `[validate]` counts as empty |
| `settings/unknown-key` | error | a key inside `[validate]` that the shape does not have (`[validate.inworld_vocab]`) | the key | none; an error, because a misspelt table would silently drop the project's words |
| `settings/guardrail` | error | a `severity` entry names a guardrail (CE-14.1) | the key | none: the guardrail stays an error |
| `settings/fixed` | error | a `severity` entry names a rule that has no setting | the key | none |
| `settings/unknown-rule` | warning | a `severity` entry names no rule | the key | none |
| `settings/severity` | error | a value other than `error` or `warning` | the value | none |
| `settings/allow-reason` | error | an `allow` entry without a reason, or with an empty one (CE-15.2) | the entry | none: the word stays a developer word |
| `settings/word` | error | a word in `words` or `allow` that is empty or is more than one word | the item | none |

The message of CE-14.1, for an entry `"lore/never-confirm" = "warning"` on line 4:
".theos/settings.toml:4:1: error settings/guardrail: lore/never-confirm is a guardrail: it always
blocks, and no setting lowers it."

**Configured severities** apply when findings are read, through `findings()`, `findings_for` and
`preview`. The store keeps the default severities (P03 §5.10), and `dump()` shows them, so a
settings change needs no rebuild. The function that applies a severity has no path that changes
a guardrail: a guardrail finding is an error whatever it is given.

Without a settings file, the defaults apply and there is no finding.

### 5.13 Binary references (CE-15.4)

- Every value of a `BinaryRefs` field is a binary reference `path@sha256:<hex>`, relative to the
  binaries folder (G10, data model §10). Today these are the versions' `outputs`; P26's style
  `refs` join without new code (CE-68.3).
- `Index::binary_findings(binaries_root)` checks them on request. It stats each file and hashes
  it, streaming, only when its size or time differ from a hash memo in the cache, keyed by folder,
  path, size and time:
  - `binary/missing` (warning): no file at the path, or the path leaves the folder (`..`, an
    absolute path, a drive or UNC prefix). Such a path is never read.
  - `binary/hash` (warning): the file's SHA-256 differs from the reference.

  Both sit at the reference value.
- These findings are not stored with the others: the binaries folder is per machine and outside
  the project, so the shared cache stays the same for every process and machine. The memo is not
  part of `dump()`.
- P05's `theos check` asks for them when a binaries folder is set, and otherwise says that binaries
  were not checked.

### 5.14 The Shared Formalism mirror (CE-16)

**The format of `_source.json`** is P04's, as its only reader. The private session of CE-16S
writes it in this format.

```json
{
  "format": 1,
  "page": "https://…",
  "equations": [
    { "id": "E1", "title": "…", "anchor": "https://…#…", "text": "…" }
  ]
}
```

- `format`: 1.
- `page`: the URL of the Shared Formalism page.
- `equations`, in page order, each with:
  - `id`: `E` and the number Shared Formalism gives it (data model §1), unique, without leading
    zeros;
  - `title`: one line;
  - `anchor`: the URL of the equation on the page (P03's `Url` rule);
  - `text`: the equation as the page writes it (Markdown, with its LaTeX), any number of lines.
- The number of equations is the source's. P04 does not fix it at 18; the synthetic fixture has
  18 (CE-16.1).

**The generated record** `canon/formalism/<id>.md`:

```markdown
---
# generated from canon/formalism/_source.json by theos; a hand edit fails the check
id: E7
kind: equation
title: <title>
source: <anchor>
generated: true
---
<text>
```

- Scalars are written by P02's YAML encoder, quoted where they need it. LF, with a final newline.
  A CR in the source becomes LF.
- `formalism::render(source)` gives the files. `formalism::regenerate(root)` writes each file
  whose bytes differ and nothing else. It never deletes. A second run writes nothing (CE-16.4).
  `theos formalism` calls it (§5.15).

**Checks.** Subjects: equation records and `_source.json` (§5.5).

| Rule | Severity | Finding when | On |
|---|---|---|---|
| `formalism/edited` | error | an equation record's bytes differ from what the source renders for its id (CE-16.3): SHA-256 of the rendering against the stored file hash | the `generated` value, or 1:1 without one |
| `formalism/stray` | error | an equation record the source does not list, or any equation record when there is no source | the `id` value |
| `formalism/missing` | error | an equation of the source without its record | `_source.json` at 1:1, naming the id |
| `formalism/source` | error | `_source.json` is not valid JSON or does not have the format | `serde_json`'s line and column, else 1:1 |

An equation record reads `rec:` of itself and `file:canon/formalism/_source.json`; the source
reads `rec:E<n>` of each id.

- The comparison is byte for byte. A CRLF copy counts as edited; `.gitattributes` in
  tower-of-theos forces LF for Markdown (CE-01.2).
- **The equation body is verbatim.** It is the page's text, LaTeX included, not Markdown to
  judge. The `equation` kind's `BodyGrammar` says so. Its body gives no body findings, no
  mentions and no in-world text, so a `[[` in LaTeX cannot become a link finding.
  `formalism/edited` judges the body.

CE-16.2 needs no new rule: once `E7.md` exists, P03's resolution resolves `cites: [E7]`.

### 5.15 `theos ledger` and `theos formalism`

The `theos` binary gets its first two commands, with `clap` (derive) and `anyhow`. P05 adds the
rest and `--json` (CE-17.1).

- `theos ledger [<folder>]` opens the project (the current folder by default), syncs the index
  and prints the ledger, every truth by id:

  ```text
  T-940  Synthetic statement A  (target confirmed)
    hinted     F04 · T1 · C-940
    suspected  F21 · T2 · C-941
    confirmed  F57 · T2 · C-942
    placed by  S-940#2
  ```

  One line per clue, in the order of §5.7; `F?` for a clue without a floor; "no clue" for a truth
  without one. Exit code 0; 1 when the project does not open.
- `theos formalism [<folder>]` regenerates the mirror and prints each path it wrote, or "nothing
  to write". Exit code 1 when `_source.json` is missing or does not have the format.

### 5.16 The store

New tables (indicative; the plan names the columns): `inputs` (path, hash, text), `notes`,
`inworld`, `deps` (subject, key), `ledger`, and the binary hash memo. `beats` gains the `when::`
token. The format version goes to 3.

`dump()` adds `notes`, `inworld`, `inputs` (without text), `deps`, `ledger` and the lore
findings. It leaves out the binary memo. `deps` is in the dump on purpose: reads that differ
from a cold build's would miss a later change, so the history test compares them too.

### 5.17 Fixtures

All fixtures are synthetic and say so: `# synthetic fixture (P04), not canon` in the front
matter, and neutral texts ("Synthetic statement A"), never lore.

- **Fixture projects** in `crates/theos-content/tests/fixtures/projects/<name>/`, each with its
  own `theos.toml` and `expected.toml`:
  - `body/`: every body rule of §5.3, passing and failing, also in a kind whose grammar gives the
    construct no meaning;
  - `lore/`: for each of the six lore rules a passing and a failing part (CE-15.1); never-confirm
    and no-overshoot on a clue and on a beat; the expected ledger;
  - `world/`: floors and bands, locations under locations, `floor-number`, parent cycles,
    broken chains;
  - `story/`: the four story rules, holders whose lenses reject;
  - `versions/`: `picked`, `from`, `addresses`, notes and their states;
  - `settings/`: settings files with every row of §5.12;
  - `formalism/`: a synthetic `_source.json` with 18 equations `E1`…`E18` ("Synthetic equation
    7", anchors under `https://example.invalid/synthetic-formalism`) and a truth citing `E7`;
  - `binaries/`: a binaries folder with a matching, a changed and a missing file, and a path that
    leaves the folder.
- **The P02 corpus project.** P03's `expected-index.toml` lists every finding of the project
  (P03 §5.13), so it gains the findings P04's rules make there: the `malformed/` records'
  `body/` findings among them. P03's other expectation files gain theirs the same way. No P03
  expectation is dropped.
- **The CE-12.1 project** keeps P03's size, shape and limit: 500 records, 25 per kind, bodies of
  about 1 KB, the same links, under one second. Its generated data is kept valid under the rules
  of the current build, so its test still asserts no findings; P04 adds a synthetic
  `_source.json` with the equations rendered from it, gives each band the floor range of its
  floor, takes ids out of belief titles, and gives the truths and clues a valid reveal order.
  The lore step runs on its 500 records.
- **The history test** (`tests/index/changes.rs`, proptest) gains truths, clues, bands, floors,
  locations, scripts with `reveals::`, concepts and versions with notes, `.theos/settings.toml`
  and `_source.json`. Its edits:
  - change a clue's `moves_to`, `reveals`, `where` or `after`;
  - change a truth's `target` or `requires`;
  - change a floor's `number` or `parent`, or a band's `floors`;
  - add or remove a beat's `reveals::`, or a clue;
  - rename a truth;
  - close and open a parent cycle;
  - add a developer word;
  - change an equation in the source, or hand-edit an equation record.

  After every step, `dump()` equals a cold build of the same files.

### 5.18 How each AC is proven

Tests carry `// AC: <id>` directly above a `#[test]` function (P00's `ac-trace`).

| AC | Proven by |
|---|---|
| CE-14.1 | In `settings/`: a `severity` entry for each guardrail, set to `warning`, gives `settings/guardrail` with path, line, column and the message of §5.12, and that guardrail's findings stay errors. An entry for an order rule changes that rule's severity on read, with no rebuild (`dump()` unchanged). A unit test gives the severity function every value for every guardrail: each comes back an error. |
| CE-14.2 | Each fixture project's findings equal its `expected.toml`: rule, severity, path, line and column. A test walks `rules()` and asserts that each rule P04 adds has at least one finding in some fixture, and that every finding has a rule, a severity, a path and a line, also those on `.theos/settings.toml` and `_source.json`. |
| CE-14.3 | Changes, each checked with `update` and with `preview`: a band's `floors` (its floor gets `world/floor-number`); a clue's `moves_to` (its truth's ledger and three-clue, and the beats that reveal it); a truth's `target` (its clues and their beats); a note removed from a concept (the version that addresses it); a floor moved to another band (the ledgers of truths with clues below it); a word added to the settings (the records with in-world text). Each `Update.revalidated` lists exactly the expected paths; each preview returns their findings and changes no row. After each change, `dump()` equals a cold build. The history test of §5.17. |
| CE-15.1 | In `lore/`: for each of the six rules, the passing part has no finding of that rule, and the failing part has exactly its expected findings, with rule, path, line and column. never-confirm and no-overshoot fail both on a clue and on a beat. |
| CE-15.2 | In `settings/`: an `allow` entry without `reason`, and one with `reason = "  "`, each give `settings/allow-reason` (error) at the entry, and the word stays a developer word. An entry with a reason exempts it. |
| CE-15.3 | The `lore/` project's `ledger()` equals its expected ledger, from a cold build and after the changes of CE-14.3; a second open that reads no file returns the same ledger from the cache. `theos ledger` on that project prints the expected text (a test runs the binary). |
| CE-15.4 | In `binaries/`: a missing file gives `binary/missing` (warning) at the reference; a changed file gives `binary/hash` (warning); a matching file gives nothing; a path that leaves the folder gives `binary/missing` and is not opened. A second call hashes no file. |
| CE-15.5 | An unknown note state gives `body/note-state` (error) at the state; an `addresses` item naming a note the parent lacks gives `version/addresses` (error) at the item; one naming an existing note gives nothing. |
| CE-16.1 | `render` of the synthetic source gives `canon/formalism/E1.md` … `E18.md` with the expected bytes; `regenerate` writes them; the index then holds 18 equation records with no finding. |
| CE-16.2 | In `formalism/`, a truth with `cites: [E7]` has no link finding; with `E7.md` removed, it has `link/dangling`. |
| CE-16.3 | An edit of `E7.md` (its title, its body, one added space) gives `formalism/edited` (error); `regenerate` repairs it. |
| CE-16.4 | `regenerate` twice: the second writes nothing, and every byte is unchanged. The same after a hand edit and one repairing run. |

Untagged tests cover the rest of §5.5: the reads a cold build stores equal those of an incremental
history; a subject that reads an id that does not exist yet re-runs when that id appears; a
removed subject leaves no reads, ledger rows or findings.

The gate as merged applies: no `allow` or `warn` attributes, a reason on every `expect` and
`ignore`, 80 % line coverage of library crates and 60 % of `theos`, error paths included,
`missing_docs`. CE-12.1 stays green: the plan measures the lore step of a cold 500-record build
and of a one-file change, and records the numbers. Binary paths and file stamps differ per OS
(drive and UNC prefixes, time resolution), so the build PR gets the labels `windows` and `macos`
and links their runs. `cargo xtask gate` runs on Windows before the phase is claimed done
(CLAUDE.md).

## 6. Interfaces other phases use

| Phase | Uses |
|---|---|
| P05 | `findings()` with configured severities; `binary_findings(root)`; `ledger()` and `ledger_of` for `theos ledger --json` and `ledger_check` (CE-18.1); `rules()` with classes; `theos ledger` and `theos formalism`, which P05 extends; `preview` with neighbours for the hook (CE-17.2) |
| P06 | `Preview.findings` with the neighbours (CE-14.3) to reject a command (CE-20.1); blocking means error after configuration, and guardrails are always errors |
| P07 | The same for proposals (CE-26.3); `generated: true` and the `equation` kind to lock the mirror, truth `target` to lock `never` truths (architecture §9) |
| P09 | The `notes` rows (id, state, version) for open notes in run requests (CE-29.6) |
| P11, P12 | Body findings and their positions while typing, through `preview` (CE-35.2); in-world segments for flagging developer words inline (CE-35.3) |
| P14 | `rules()` and classes for the settings form; `[validate]` of `.theos/settings.toml`; guardrails have no setting (CE-39.2); `preview` of a settings change |
| P15 | never-confirm on the beat (CE-41.2); `ledger_of` for the truth screen; reveal sites |
| P17 | `preview` of a draft runs every lore rule (CE-46.3) |
| P21 | Rules registered by parts; `BodyGrammar` with the step attributes; quest steps as reveal sites with per-path checks (CE-58.3); the cycle walk (CE-58.6) |
| P26 | `BinaryRefs` checks for style `refs` (CE-68.3); the cycle walk for `extends` |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| A rule reads something not through the context | The cache differs from a cold build after some change | Rules get no connection; `deps` is in `dump()`; the history test compares after every step |
| The lore step slows the cold open | CE-12.1 fails on the CI runners (coverage and Windows ran about 3× slower than local in P03) | Bulk loading for a cold build; the plan measures and records the lore step; the 500-record test keeps its size, shape, limit and "no findings" assertion |
| Coarse reads re-run many subjects | A prose edit of a band re-runs every truth with a clue below it | Correct; the counts are small; narrowing is allowed only with the history test equal (§5.5) |
| The rule readings differ from what Jann or the lore pages mean | Findings that are wrong for the canon | Every reading is a ruling in §9; the private session can compare them with the lore pages |
| Developer words flag ordinary words | Noise in in-world text | Built in are only ids, tiers and rule names; words come from the project; allowlist entries need reasons |
| Settings file broken by an unrelated table | The project's words and severities are lost | `settings/toml` is an error, so the hook and CI block until it is fixed; guardrails need no setting |
| `_source.json` does not exist until CE-16S | No real mirror | The synthetic fixture proves the mechanism; `docs/status.md` names the format |
| Large binaries | Slow hashing | Stamps first, a hash memo, streaming |
| Directive 3: rule readings keep leaking in reviews | Review rounds without end | The rules sit on one context and one definition of availability (§5.7); if reviews keep finding new variants, the class gets named and the definition revisited |

## 8. Review Focus — inputs most likely to break it

- **Places:** a clue `where` naming a band, a floor, a deep location; a location under a
  location; a chain that dangles, has a wrong kind, or loops; a floor whose parent is not a band;
  a band `floors: [1, 20]` with floor 21; a floor without a valid `number`.
- **Truths and clues:** target `never` with `moves_to: hinted`; target `hinted` with `moves_to:
  suspected`; a `reveals` to a dangling id, to a non-truth, to a duplicated id; `target: maybe`;
  a clue without a valid `gate`; a clue in two places on different floors.
- **Order:** `after` naming itself; a two-clue cycle; an `after` on a later floor; on the same
  floor; with a higher gate; a truth confirmed on a lower floor than it is hinted; equal floors.
- **Requires:** a truth requiring itself; a two-truth cycle; requiring a `never` truth; requiring
  a truth that is confirmed only later; a truth that never reaches its target.
- **Three-clue:** two clues; three clues with one invalid `moves_to`; three clues of a lower state.
- **Beats:** a `reveals::` of a clue at another place; `at::` missing; `when::` missing,
  `when:: T4`; two `at::` links; a `says::` rejected by one of two holders and by both; a holder
  whose chain is broken.
- **Body:** a beat heading or note callout in a topic; an unknown note state; two notes `n1`; two
  beats 3; a key twice; an unknown key; a voice tier `T4`; a malformed link inside a callout.
- **Settings:** a guardrail set to `error` and to `warning`; an unknown rule; a fixed rule; a
  value `off`; malformed TOML; an `allow` entry with an empty reason or only spaces; a word with
  a space; no settings file; CRLF.
- **In-world text:** a word inside a longer word; upper case; non-ASCII; in a link target, in
  shown text, in a bare link; a design note inside a voice block; a caption; a belief title; a
  dialogue heading; a design callout in an in-world topic.
- **Formalism:** no source; malformed JSON; an id twice; `E0`, `E07`; a title that needs quotes;
  a text with `---` or a CR; a hand edit of one space; a stray `E19.md`; a deleted `E7.md`;
  regenerating twice.
- **Neighbours:** renaming a truth; deleting a band; moving a location to another floor; a
  settings change in the middle of a history; a preview of a clue change; a subject reading an
  id that appears later.
- **Binaries:** no folder given; a missing file; a changed file with the same size; upper-case
  hex; a path with `..`, an absolute path, a drive prefix; a file replaced between stat and hash.

## 9. Rulings (Claude's calls)

- Ruling: only reveal-order, requires and three-clue have a severity setting; an entry naming any
  other rule is a finding — architecture §10 lists only these as lowerable, and the settings
  screen offers only them — cost if wrong: Jann may want to lower `link/dangling` during the
  lore import, which is one class change per rule.
- Ruling: reveal-order and requires default to error, three-clue to warning — the first two are
  consistency, three-clue is robustness, and the truth and graph screens show three-clue as a
  warning — cost if wrong: one default per rule.
- Ruling: severities are set per rule in `[validate.severity]`, although the settings screen shows
  one switch with the key `git.validate` — CE-14.1 needs a setting that can name a guardrail; P14
  maps the switch onto the three entries or aligns the key — cost if wrong: a key rename.
- Ruling: a bad settings entry is an error finding on `.theos/settings.toml` and has no effect,
  and malformed TOML makes `[validate]` empty — an error blocks commits through the hook and CI
  anyway, and the rest of the file keeps working — cost if wrong: Jann may prefer that every
  process refuses to run on a bad file.
- Ruling: never-confirm and no-overshoot are reported on the clue and on each beat that reveals
  it — the clue and the script are different records and each must change (CE-41.2 shows the
  finding on the beat) — cost if wrong: two findings for what Jann may see as one fault.
- Ruling: no-overshoot judges truths with target `hinted` or `suspected`; a clue of a `never`
  truth is never-confirm's alone — one fault, one finding — cost if wrong: none.
- Ruling: "on every path" in P04 is the climb: a clue's availability is its lowest floor and its
  gate, and A can be found no later than C when both are at most C's — the ledger orders by
  floor, and Understanding grows with the climb; quest paths are P21's — cost if wrong: order
  findings that Jann reads differently.
- Ruling: a clue placed in several places counts from its lowest floor — the player can find it
  there, and later places are fallbacks, which three-clue encourages — cost if wrong: an `after`
  that holds only at the earliest place passes.
- Ruling: reveal-order covers `after`, the order of a truth's states, and `after` cycles —
  architecture §10 calls it the order of reveals, and the UI references show both kinds of
  message — cost if wrong: one part of the rule to drop.
- Ruling: requires asks that the required truth can be confirmed no later than each clue that
  brings the requiring truth to its target, and that the required truth's target is `confirmed`
  — data model §7: "Truths that must be confirmed first"; the truth screen compares with the
  requiring truth's confirming floor ("T-001 is confirmed before F57") — cost if wrong: Jann may
  mean "before the requiring truth is revealed at all", a stricter check.
- Ruling: three-clue counts the clues whose `moves_to` is the truth's target, and a truth with no
  clue warns too — the truth screen: "Only one clue moves T-012 to confirmed. The three-clue rule
  asks for three." — cost if wrong: every new truth warns until it has three clues.
- Ruling: a `says::` belief is an error only when every `with::` holder rejects it — `rejects`
  means "beliefs of others this holder argues against", and a beat can show an argument; nobody
  present may be the one who says it — cost if wrong: a beat where the one speaker rejects the
  belief passes when someone else is present.
- Ruling: a beat without `at::` or `when::` skips the checks that need them — data model §9.4
  does not require them — cost if wrong: a beat reveals a clue with no place or tier check until
  it gets them.
- Ruling: a link to a duplicated truth id is judged against every record with that id, and a
  guardrail fires if any of them breaks it — the safe side for G8 — cost if wrong: an extra
  finding while a duplicate exists.
- Ruling: each P02 malformed kind is one `body/` rule of the same name — one rule per fault keeps
  findings and settings readable — cost if wrong: renames before P05 prints them.
- Ruling: beat numbers must be unique; their order and gaps are not judged — `S-004#3` needs one
  beat 3, and nothing in the data model asks for an order — cost if wrong: unordered beats pass.
- Ruling: an unknown beat attribute is a warning — like an unknown field (CE-11.4) — cost if
  wrong: one severity.
- Ruling: only `at::` has a link count; `with::`, `says::` and `reveals::` take several — data
  model §9.4 says "one link" only for `at::` — cost if wrong: none.
- Ruling: in-world text is voice content and captions (not titles), in-world topic bodies, belief
  titles and dialogue bodies without their level-2 headings; design callouts and `Design note:`
  lines never; a bare link's id is checked — data model §9.2 and §9.4; dialogue headings name
  outcomes (§9.5); a bare link shows its id — cost if wrong: words flagged in captions or missed
  in headings.
- Ruling: built-in developer patterns match case-sensitively, project words case-insensitively —
  `t1` is not a tier token, but `Ledger` at a sentence start is `ledger` — cost if wrong: a
  lower-case id passes.
- Ruling: an allowlist entry exempts its word everywhere; there are no entries per record —
  "allowlist entries need a reason" names no scope, and a reason per word is reviewable in one
  file — cost if wrong: a word allowed for one people is allowed for all.
- Ruling: binary findings are computed on request and not stored; the hash memo lives in the
  cache — the binaries folder is per machine — cost if wrong: P05 calls two functions instead of
  one.
- Ruling: a binary path that leaves the binaries folder is reported as missing and never opened —
  a reference names a file inside the folder (G10), and reading elsewhere would read files the
  project does not own — cost if wrong: one message instead of a rule of its own.
- Ruling: the format of `_source.json` is P04's, and the number of equations is the source's —
  P04 is its only reader; data model §1 fixes the ids' form, not their count — cost if wrong: the
  private session writes another shape, and the format changes before the real mirror.
- Ruling: regenerating writes only files whose bytes differ and never deletes; a stray equation
  record is a finding — deleting files is destructive and a stray record may be someone's work —
  cost if wrong: a stray record has to be deleted by hand.
- Ruling: a hand edit is found by comparing the SHA-256 of the rendering with the stored file
  hash — the store already holds the hash, and no record text is stored twice — cost if wrong:
  the finding sits at the `generated` value, not at the first changed line.
- Ruling: `formalism/missing` sits on `_source.json` at 1:1 — `serde_json` gives no spans for
  values — cost if wrong: a coarser position for a rare finding.
- Ruling: P04 adds `theos ledger` and `theos formalism`, text only — CE-15.3 names `theos
  ledger`, and the mirror needs a way to run it once `_source.json` exists; P05 adds `--json` and
  the other commands — cost if wrong: two small commands P05 might shape differently.
- Ruling: rules of an editor part are registered by the part, the body and settings rules in
  lists of their own — architecture §6 has parts register rules; the body rules judge P02's
  grammar for every kind — cost if wrong: a move of a few lines per rule.
- Ruling: a re-parse changes `rec:<id>` even when only prose changed — coarse reads are simple and
  correct — cost if wrong: more re-runs than needed after a prose edit.
- Ruling: a lore finding always sits on its own subject, and ledger rows belong to their truth —
  each subject then replaces exactly its own rows — cost if wrong: none.
- Ruling: `Update` gains `revalidated`, and P03's CE-12.2 expectations gain that list — CE-14.3's
  neighbours must be visible to P06 — cost if wrong: one field.
- Ruling: P03's expectation files gain the findings P04's rules make on P03's fixtures; none is
  dropped — P03 §5.13 lists every finding of a project — cost if wrong: longer expectation
  files.
- Ruling: `.theos/settings.toml` and `canon/formalism/_source.json` are input files of the index,
  checked at every sync — their contents decide stored findings — cost if wrong: two stats per
  sync.
- Ruling: reveal-order (b) compares a clue with the nearest lower state the truth has clues for —
  the order is transitive, so this keeps all states in order and gives one finding per
  out-of-order state; "any lower state" would pass a truth confirmed before it is suspected —
  cost if wrong: Jann may want every lower state reported.
- Ruling: an unknown key inside `[validate]` is an error — a misspelt table would silently drop
  the project's developer words, which weakens a guardrail — cost if wrong: a typo blocks commits
  until fixed.
- Ruling: voice titles are never in-world text, also inside in-world topics and dialogues — they
  hold the speaker, beliefs and tier — cost if wrong: none.
- Ruling: an equation's body is verbatim, so it gives no body findings, mentions or in-world text
  — it is the page's text with its LaTeX, and `formalism/edited` judges it — cost if wrong: a
  link written in an equation's text is not resolved.
- Ruling: the mirror is compared byte for byte, so a CRLF copy counts as edited — the content
  repo forces LF, and the mirror must equal its source — cost if wrong: a Windows checkout
  without `.gitattributes` reports every equation.
- Ruling: one rule id may be registered by several parts, each for its own kinds, when severity
  and class agree; a rule may check kinds of other parts — the version rules serve both concepts
  and maps, and never-confirm checks scripts — cost if wrong: none.
- Ruling: the build PR gets the labels `windows` and `macos` — binary paths (drive and UNC
  prefixes) and file stamps differ per OS — cost if wrong: two runs of `platforms`.

## 10. Left to the builder

- Module, type and table names, columns and SQL, as long as §5 and §6 hold.
- The exact messages, within the tables of §5.
- How the context loads facts in bulk for a cold build, and whether reads get narrower (§5.5).
- The layout of `theos ledger`'s text, within §5.15.
- Whether the settings rules live in the lore step or in a module of their own.
