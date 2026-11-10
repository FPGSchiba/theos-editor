# Build status

Updated by Claude at every phase transition. Not a protected path.

States: `todo` → `spec-pr` → `spec-approved` → `in-progress` → `pr-open` → `done`.

| Phase | Name | Repo | State | Spec PR | Build PR |
|---|---|---|---|---|---|
| P00 | Foundation and gate | theos-editor | done | #5, #9, #10 | #7 |
| P01 | Content layout and templates | tower-of-theos | done | #4 | tower-of-theos#2 |
| P02 | Records and grammars | theos-editor | done | #16 | #17 |
| P03 | Kinds, index, search, lenses | theos-editor | done | #18 | #20 |
| P04 | Validation, lore rules, ledger, formalism mirror | theos-editor | spec-pr | #21 | |
| P05 | CLI, hook, releases, MCP read | theos-editor | todo | | |
| P06 | Command layer and git | theos-editor | todo | | |
| P07 | Agent sessions, review, MCP propose | theos-editor | todo | | |
| P08 | Map format and terrain generator | theos-editor | todo | | |
| P09 | Generator layer and generator tools | theos-editor | todo | | |
| P10 | Prose editor research | theos-editor | done | #3 | #11 |
| P11 | Editor shell and Codex | theos-editor | todo | | |
| P12 | Manuscript editor | theos-editor | todo | | |
| P13 | Changes and Review screens | theos-editor | todo | | |
| P14 | Start and Settings | theos-editor | todo | | |
| P15 | Lenses, Scripts, Graph | theos-editor | todo | | |
| P16 | Generative screens | theos-editor | todo | | |
| P17 | AI drafts and semantic search | theos-editor | todo | | |
| P18 | Installer, first run, Code preset | theos-editor | todo | | |
| P19 | Comments | theos-editor | todo | | |
| P20 | Content layout for world and art kinds | tower-of-theos | pr-open | #22 | tower-of-theos#3 |
| P21 | World kinds: enemies, spawns, quests | theos-editor | todo | | |
| P22 | Map pins, cameras and frames | theos-editor | todo | | |
| P23 | Map recipes and blockout generator | tower-of-theos | todo | | |
| P24 | Quest screen and flow editor | theos-editor | todo | | |
| P25 | Atlas | theos-editor | todo | | |
| P26 | Style bible and briefs | theos-editor | todo | | |
| P27 | Asset pipelines | theos-editor | todo | | |
| P28 | Workflow catalogue and asset guide | tower-of-theos | todo | | |
| P29 | Settlements, interiors and child maps | theos-editor | todo | | |
| P30 | Indoor floors with stacked levels | theos-editor | todo | | |

Rows P20–P28 come from umbrella version 2 (#19). Rows P29 and P30, and the new P08 and P23, come
from umbrella version 3 (decision 0003), which waits for Jann in its docs PR (#24).

## Needs Jann

- Decision 0003, the map generator in Rust (umbrella version 3, `docs/map-package.md`), waits for
  Jann in its docs PR #24. The gate PR #23, which moved one assertion of an xtask test from P08
  to P20 for it, is merged. P08's spec starts once #24 is merged.
- Before P08's build, one more gate PR (decision 0003 §12) adds `theos-map-format` and
  `theos-maps` to the workspace test and `gate/thresholds.toml`, and bans the std float methods in
  `gate/clippy.toml`. P08's spec lists its contents.
- The Maps screen reference (`docs/ui/screens/11-maps.html`) still says "theos_maps 0.3.0 ·
  Python 3.12 (uv)" and describes a Python venv. P16's spec PR updates it (G18).
- P00's macOS `platforms` evidence is still open: the run on #7 got no GitHub runner (macOS
  capacity). The weekly Monday run, or a re-run, gives it.
- Comments on text sections (decision 0002, #15, phase P19): before P19 starts, Jann settles the
  wording for G7 and CE-23.1 proposed in 0002 §10, which P19's agent tools need. P02 builds the
  UTF-16 positions 0002 asks for.
- Three sentences of the P10 corpus read like lore claims (B-901's link text, "the Cutter is
  near", "the operator-plate names a Listener"). P02's fixture copies reword them neutrally until
  Jann, or a session that can read Confluence, confirms they are canon.
- P03 spec §5.7 (a protected path) says only that hidden entries are skipped. Jann ruled on
  2026-11-10 that hiding is native, and the build follows that:
  - on Windows, a file or folder is hidden when it has the hidden attribute;
  - on Linux and macOS, a name that starts with a dot is hidden;
  - on every OS, `.theos`, `.git`, `.obsidian`, `.vscode`, `.github`, `.worktrees` and `.trash`
    are never records.

  Two consequences follow. A file hidden by its attribute on Windows is a record on Linux. On
  Windows, a dot folder outside that list and without the attribute (`.claude/`, `.idea/`) is
  walked unless `.gitignore` ignores it; in tower-of-theos today that changes nothing. The spec
  text can be aligned in a docs PR.
- P03 spec §7's risk row for two builds sharing one cache (a protected path) says each open
  rebuilds. Since the final review, every write rechecks the cache identity: a build rebuilds the
  cache whenever another build has written since, and after three rebuilds in one write it fails
  with `IndexError::Contended`. The row can be aligned in a docs PR.
- P02 spec §5.11 says a design note is "a paragraph whose text starts with `Design note:`". The
  build follows data model §9.2 instead ("on a line starting with `Design note:`"), which ranks
  higher: a note line inside a paragraph starts a note that runs to the paragraph's end. The spec
  wording can be aligned in a docs PR.
- `CLAUDE.md` ("Work there only in the phases that name it (P01, P08)") and the theos-phase skill
  ("Phases in tower-of-theos (P01, P08)") predate umbrella versions 2 and 3: version 2 adds P20,
  P23 and P28 in tower-of-theos, and version 3 (decision 0003) moves P08 to theos-editor. The
  decision 0003 docs PR aligns both with the umbrella's repo column; the entry can go once it is
  merged.
- Data model §14 contradicted itself, found by the P20 spec: its pins example placed pins on
  `f04-terrain-v1`, whose example had no `frame`. The decision 0003 docs PR gives that example a
  `frame` (and `base` and `features` outputs); the entry can go once it is merged.
- The P20 build PR tower-of-theos#3 waits for Jann (content repo). Its final review loop ended
  clean in round 3. Its description lists four manual checks in Obsidian and VS Code. It locks
  `markdown-it-py` 4.3.0, which was released the day it was locked. Pinning an older 4.x is a
  one-line follow-up if Jann prefers a cool-down.

## Needs a private session (Confluence, Jira, plugin)

- CE-16S: write `tower-of-theos/canon/formalism/_source.json` from Shared Formalism, in the
  format of the P04 spec §5.14, then run `theos formalism` to generate `E1`…`E18`. P04 proves
  the mirror with a synthetic source until then.
- inworld-vocab: if Confluence or the theos-design plugin holds a list of developer words, copy
  it into `words` of `[validate.inworld-vocab]` in `tower-of-theos/.theos/settings.toml`
  (P04 spec §5.11, §5.12). Built in are only ids, tier tokens and rule names.
- CE-04 / CE-05: lore import and Seal split (umbrella §4).
- Readable copies in Confluence: the umbrella spec (version 2), the data model and the
  architecture page need the world and art additions once the docs PR is merged.
- Decision 0003 (map generator in Rust), once merged: the readable copies of the umbrella spec
  (version 3), the architecture, the data model and the generator protocol; a decision page for
  0003; and Confluence Decision 221741058 (the engine package format), which `docs/map-package.md`
  replaces (Jann confirmed on 10 Nov 2026 that it fixes nothing different).
- The `theos_maps` skill in Jann's other Claude account: once P09 is merged, point it at the
  editor's map generator (`theos gen run` in a shell, or `gen_run` over MCP) or retire it; its
  recipes move into the content project in P23.
- CE-75: move the style bible (`style.yaml` v2 and its reference crops) from Drive into `art/`
  style records, after P20.
- Jira: slices S8 and S9 have no epics yet (create only on Jann's explicit yes).
