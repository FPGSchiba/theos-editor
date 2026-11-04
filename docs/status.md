# Build status

Updated by Claude at every phase transition. Not a protected path.

States: `todo` → `spec-pr` → `spec-approved` → `in-progress` → `pr-open` → `done`.

| Phase | Name | Repo | State | Spec PR | Build PR |
|---|---|---|---|---|---|
| P00 | Foundation and gate | theos-editor | spec-pr | #5 | |
| P01 | Content layout and templates | tower-of-theos | pr-open | #4 | tower-of-theos#2 |
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

- Data model §3/§4: a slug id such as `true`, `false` or `42` matches the id pattern, but as an
  unquoted plain scalar it is not text. Should such ids be forbidden by the pattern, or should ids
  always be read as text? Needed by P02/P03 (found in the P01 spec, §5). The same holds for
  `accuracy: false` on beliefs, which the restricted YAML reads as a boolean; the P01 checks
  accept it as the enum value for now.
- P01 build PR tower-of-theos#2 is a draft labelled `needs-jann`: the final review loop reached its
  cap of 5 rounds. Two Minor findings are open (listed in the PR): the README's Obsidian steps do
  not name `number: 1` and `floors: [1, 1]` as values to replace, and the sample id `f100-summit`
  implies lore. Manual checks before merging: open the vault in Obsidian 1.14 and see `git status`
  stay clean; make one record with *Insert template* and one with a VS Code snippet.
- P00 revision 1 (spec PR from `spec/P00-gate-redesign`): the checks read code through the
  compiler, cargo and real parsers (P00 spec §4.3, §5.10), and per-PR checks run on Linux only
  while Windows joins macOS in the `platforms` workflow (D8, umbrella G2). The build PR #7 waits
  for it: the plan is appended to the P00 plan and built on #7 once this is approved.
- Docs PR #8: the engineering directives in `CLAUDE.md`. Its checks run once P00 is merged.

## Needs a private session (Confluence, Jira, plugin)

- CE-16S: write `tower-of-theos/canon/formalism/_source.json` from Shared Formalism before P04.
- CE-04 / CE-05: lore import and Seal split (umbrella §4).
