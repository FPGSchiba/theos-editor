# Build status

Updated by Claude at every phase transition. Not a protected path.

States: `todo` → `spec-pr` → `spec-approved` → `in-progress` → `pr-open` → `done`.

| Phase | Name | Repo | State | Spec PR | Build PR |
|---|---|---|---|---|---|
| P00 | Foundation and gate | theos-editor | todo | | |
| P01 | Content layout and templates | tower-of-theos | todo | | |
| P02 | Records and grammars | theos-editor | todo | | |
| P03 | Kinds, index, search, lenses | theos-editor | todo | | |
| P04 | Validation, lore rules, ledger, formalism mirror | theos-editor | todo | | |
| P05 | CLI, hook, releases, MCP read | theos-editor | todo | | |
| P06 | Command layer and git | theos-editor | todo | | |
| P07 | Agent sessions, review, MCP propose | theos-editor | todo | | |
| P08 | Map toolset as a generator | tower-of-theos | todo | | |
| P09 | Generator layer and generator tools | theos-editor | todo | | |
| P10 | Prose editor research | theos-editor | in-progress | #3 | |
| P11 | Editor shell and Codex | theos-editor | todo | | |
| P12 | Manuscript editor | theos-editor | todo | | |
| P13 | Changes and Review screens | theos-editor | todo | | |
| P14 | Start and Settings | theos-editor | todo | | |
| P15 | Lenses, Scripts, Graph | theos-editor | todo | | |
| P16 | Generative screens | theos-editor | todo | | |
| P17 | AI drafts and semantic search | theos-editor | todo | | |
| P18 | Installer, first run, Code preset | theos-editor | todo | | |

## Needs Jann

- P00 spec: `ac-trace` has to find `// AC:` tags in the TypeScript tests under `research/**`, or
  P10 cannot pass it (P10 spec §5).

## Needs a private session (Confluence, Jira, plugin)

- CE-16S: write `tower-of-theos/canon/formalism/_source.json` from Shared Formalism before P04.
- CE-04 / CE-05: lore import and Seal split (umbrella §4).
