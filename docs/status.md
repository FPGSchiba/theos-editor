# Build status

Updated by Claude at every phase transition. Not a protected path.

States: `todo` → `spec-pr` → `spec-approved` → `in-progress` → `pr-open` → `done`.

| Phase | Name | Repo | State | Spec PR | Build PR |
|---|---|---|---|---|---|
| P00 | Foundation and gate | theos-editor | done | #5, #9, #10 | #7 |
| P01 | Content layout and templates | tower-of-theos | done | #4 | tower-of-theos#2 |
| P02 | Records and grammars | theos-editor | done | #16 | #17 |
| P03 | Kinds, index, search, lenses | theos-editor | spec-pr | | |
| P04 | Validation, lore rules, ledger, formalism mirror | theos-editor | todo | | |
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

## Needs Jann

- P00's macOS `platforms` evidence is still open: the run on #7 got no GitHub runner (macOS
  capacity). The weekly Monday run, or a re-run, gives it.
- Comments on text sections (decision 0002, #15, phase P19): before P19 starts, Jann settles the
  wording for G7 and CE-23.1 proposed in 0002 §10, which P19's agent tools need. P02 builds the
  UTF-16 positions 0002 asks for.
- Three sentences of the P10 corpus read like lore claims (B-901's link text, "the Cutter is
  near", "the operator-plate names a Listener"). P02's fixture copies reword them neutrally until
  Jann, or a session that can read Confluence, confirms they are canon.
- R5-1, open since #17 merged, is fixed in P03's build (plan Task 1).
- P02 spec §5.11 says a design note is "a paragraph whose text starts with `Design note:`". The
  build follows data model §9.2 instead ("on a line starting with `Design note:`"), which ranks
  higher: a note line inside a paragraph starts a note that runs to the paragraph's end. The spec
  wording can be aligned in a docs PR.

## Needs a private session (Confluence, Jira, plugin)

- CE-16S: write `tower-of-theos/canon/formalism/_source.json` from Shared Formalism before P04.
- CE-04 / CE-05: lore import and Seal split (umbrella §4).
