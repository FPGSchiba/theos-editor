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
| P08 | Map toolset as a generator | tower-of-theos | todo | | |
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
| P20 | Content layout for world and art kinds | tower-of-theos | in-progress | #22 | |
| P21 | World kinds: enemies, spawns, quests | theos-editor | todo | | |
| P22 | Map pins, cameras and frames | theos-editor | todo | | |
| P23 | Map outputs for the atlas, blockout generator | tower-of-theos | todo | | |
| P24 | Quest screen and flow editor | theos-editor | todo | | |
| P25 | Atlas | theos-editor | todo | | |
| P26 | Style bible and briefs | theos-editor | todo | | |
| P27 | Asset pipelines | theos-editor | todo | | |
| P28 | Workflow catalogue and asset guide | tower-of-theos | todo | | |

Rows P20–P28 come from umbrella version 2 (#19).

## Needs Jann

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
  ("Phases in tower-of-theos (P01, P08)") predate umbrella version 2, which adds P20, P23 and P28
  in tower-of-theos. P20 follows the umbrella, which ranks higher. Both files are protected paths
  and can be aligned in a docs PR.
- Data model §14 contradicts itself, found by the P20 spec. Its pins example places pins on
  `f04-terrain-v1`, but that version's own example has no `frame`, and §8 says "A version without
  a `frame` takes no pins". Giving the example version a `frame` would align it. P20's tests do not
  run the pin checker on that fragment, and P22 will meet the same example.

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
- CE-75: move the style bible (`style.yaml` v2 and its reference crops) from Drive into `art/`
  style records, after P20.
- Jira: slices S8 and S9 have no epics yet (create only on Jann's explicit yes).
