# Build status

Updated by Claude at every phase transition. Not a protected path.

States: `todo` → `spec-pr` → `spec-approved` → `in-progress` → `pr-open` → `done`.

| Phase | Name | Repo | State | Spec PR | Build PR |
|---|---|---|---|---|---|
| P00 | Foundation and gate | theos-editor | pr-open | #5, #9, #10 | #7 |
| P01 | Content layout and templates | tower-of-theos | done | #4 | tower-of-theos#2 |
| P02 | Records and grammars | theos-editor | todo | | |
| P03 | Kinds, index, search, lenses | theos-editor | todo | | |
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

## Needs Jann

- Data model §3/§4: a slug id such as `true`, `false` or `42` matches the id pattern, but as an
  unquoted plain scalar it is not text. Should such ids be forbidden by the pattern, or should ids
  always be read as text? Needed by P02/P03 (found in the P01 spec, §5). The same holds for
  `accuracy: false` on beliefs, which the restricted YAML reads as a boolean; the P01 checks
  accept it as the enum value for now.
- P00 build PR #7 is a draft labelled `needs-jann`. Spec revisions 1 (#9) and 1a (#10) are built
  on it (Part 2 of `docs/plans/P00-foundation-and-gate.md`), and its final review loop runs again
  for them. Once the loop is clean, `cargo xtask github apply` needs Jann's go-ahead (the required
  check names lose their OS suffix, and the label `windows` is new). Jann adds `approved` only
  once Claude reports the branch final.
- Comments on text sections (Jann, 2026-11-04: "an important feature") are not in the umbrella.
  They need a CE in `docs/specs/00-umbrella.md` and a data-model decision on where comments live:
  a sidecar file with offsets and a text quote, or markers inside the Markdown. The P10 probe in
  `research/P10-prose-editor/app/src/probes/comments/` shows what each editor needs.

## Needs a private session (Confluence, Jira, plugin)

- CE-16S: write `tower-of-theos/canon/formalism/_source.json` from Shared Formalism before P04.
- CE-04 / CE-05: lore import and Seal split (umbrella §4).
