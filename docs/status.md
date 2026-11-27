# Build status

Updated by Claude at every phase transition. Not a protected path.

States: `todo` → `spec-pr` → `spec-approved` → `in-progress` → `pr-open` → `done`.

| Phase | Name | Repo | State | Spec PR | Build PR |
|---|---|---|---|---|---|
| P00 | Foundation and gate | theos-editor | done | #5, #9, #10 | #7 |
| P01 | Content layout and templates | tower-of-theos | done | #4 | tower-of-theos#2 |
| P02 | Records and grammars | theos-editor | done | #16 | #17 |
| P03 | Kinds, index, search, lenses | theos-editor | done | #18 | #20 |
| P04 | Validation, lore rules, ledger, formalism mirror | theos-editor | done | #21 | #28 |
| P05 | CLI and hook | theos-editor | done | #35, #41 | #42 |
| P06 | Command layer and repo lock | theos-editor | done | #36 | #39 |
| P07 | Agent sessions, review, MCP propose | theos-editor | todo | | |
| P08 | Map format and terrain generator | theos-editor | done | #27 | #44 |
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
| P20 | Content layout for world and art kinds | tower-of-theos | done | #22 | tower-of-theos#3 |
| P21 | World kinds: enemies, spawns, quests | theos-editor | todo | | |
| P22 | Map pins, cameras and frames | theos-editor | todo | | |
| P23 | Map recipes and blockout generator | tower-of-theos | todo | | |
| P24 | Quest screen and flow editor | theos-editor | todo | | |
| P25 | Atlas | theos-editor | todo | | |
| P26 | Style bible and briefs | theos-editor | todo | | |
| P27 | Asset pipelines | theos-editor | todo | | |
| P28 | Workflow catalogue and asset guide | tower-of-theos | todo | | |
| P29 | Settlements and child maps | theos-editor | todo | | |
| P30 | Levels in the generator | theos-editor | todo | | |
| P31 | Floor frame and places | theos-editor | todo | | |
| P32 | Recipe families | theos-editor | todo | | |
| P33 | Map metrics and batch tool | theos-editor | todo | | |
| P34 | Interiors | theos-editor | todo | | |
| P45 | Atlas level switcher | theos-editor | todo | | |
| P46 | Git read | theos-editor | todo | | |
| P47 | Git write | theos-editor | todo | | |
| P48 | MCP read | theos-editor | spec-pr | branch `spec/P48-mcp-read-2` | |
| P49 | Releases and the check action | theos-editor | todo | | |
| P50 | Leaner CI | theos-editor | pr-open | #45, #46 | draft, branch `phase/P50-leaner-ci-2` |

Rows P20–P28 come from umbrella version 2 (#19); P29, P30 and the new P08 and P23 from version 3
(decision 0003, #24). Rows P31–P34 and P45, and the new P29 and P30, come from umbrella version 4
(decision 0004, #31). Rows P46 and P47 and the new P06 come from umbrella version 5 (the split of
P06, #33). Rows P48 and P49 and the new P05 come from umbrella version 6 (the split of P05,
#34). Row P50 comes from umbrella version 7 (#45), amended by version 8.

## Needs Jann

- P08's build PR #44 is a draft labelled `needs-jann`. The final review loop reached its cap of
  five rounds with 6 Minor findings open. A targeted fix and review round, which Jann asked for
  on 25 Nov 2026, closed them. Its spec changes reached main with #40 (rulings 51–75) and #47
  (76–77), its gate change with #43. macOS stays unverified until the `platforms` run on #44's
  head.
- The SIL OFL 1.1 fonts (Inter, Spectral) that `theos-maps` embeds must ship with their licence
  texts (P08 §5.11, §7). `theos_maps::render::font_licences` hands them out; P09 or the release
  workflow adds them to the releases of the binaries that embed the crate.
- GitHub refused every Actions job from 13 Nov 2026 to 25 Nov: the included $18 was used up on
  3–12 Nov. Jann set an Actions budget of $15 on 25 Nov. P50 (Leaner CI) cuts the gate's
  minutes.
- P50's build PR (a draft labelled `needs-jann`, branch `phase/P50-leaner-ci-2`) waits for Jann:
  - The final review loop reached its cap of 5 rounds with one Important open (R5-I1): when a
    pull request's base changes, `ci.yml` does not run again, so a docs-only verdict made
    against the old base stays green. Fixing it changes spec §5.3's "changed paths" (decide
    against `main` instead of the event's base); the PR lays out the options.
  - Spec §5.5 lists the cached-xtask key's inputs without the root `Cargo.toml`, which the build
    hashes too ("every input" binds); a one-word spec fix.
  - When Jann approves the PR, he switches the live ruleset to the required checks `gate`,
    `test`, `coverage`, `pr` and `approval` (`cargo xtask github apply`); open PRs merge `main`
    afterwards.
  - The probe PRs, the speed measurements and the manual Intel run (spec §5.9, §5.10) follow
    Jann's ruling on R5-I1.
  - The abandoned first branch `phase/P50-leaner-ci` (its plan commit has the scope `plan`,
    which `commit-msg` refuses) can be deleted.
- Follow-up from P06: the class "a command changes which files are records without the verdict
  judging it" (m4-4, m5-1, and N1–N3 of PR #39) needs a general design. One option is a check
  after the write that compares the reindex with the preview and heals the cache; another is a
  P03 preview over the planned disk state. Rulings R16, R21 and R23 refuse the common cases
  today. R24–R26 accept the rare rest: a `.gitignore` behind a symbolic link, a hidden file
  restored visible by a Windows rollback, and new folders that differ only in case. The commit
  check stops any error those cases leave.
- For P49 (umbrella version 6):
  - before its gate PR: Jann creates a GitHub App for release-plz (contents and pull requests
    write), installs it on this repository and stores its id and key as secrets; the tag ruleset
    names it as the only actor that creates `v*` tags;
  - after the first release: Jann allows access to this repository's actions from his private
    repositories, adds a read-only token for this repository's releases as a tower-of-theos
    secret, and adds the check job to tower-of-theos;
  - Jann's first release, `v0.1.0-rc.1`, is the end-to-end proof of CE-08R.2–08R.4.
- Deferred by Jann on 12 Nov 2026 (decision 0004):
  - the shape of regions with a recommended power (D11): areas inside the floor's map (the
    working assumption) or separate maps; the power scale (working assumption: 1–5, like
    `threat`); authored in the map body or generated as a proposal;
  - who writes the band recipe families (D13); they live in `tower-of-theos`
    `tools/maps/recipes/`;
  - the research-track phases of umbrella §5 (P35–P40, P42–P44, requirements and trial sites,
    promotion at road junctions, regions and power), each until its sources are read and its
    probe has run.
- For P09's spec (decision 0004): the machine-aware memory budget for map runs (at least 3 GiB,
  more where the machine allows) covers all seeds running at once, so the built-in adapter starts
  only as many seeds in parallel as fit.
- The Maps screen reference (`docs/ui/screens/11-maps.html`) still says "theos_maps 0.3.0 ·
  Python 3.12 (uv)", describes a Python venv, and shows "2 extractions" and "extraction ↔
  extraction 99 min". P16's spec PR updates it to the built-in generator and the place types
  `chime`, `gate-up`, `gate-down` (decision 0004; G18). The Atlas tower reference
  (`13-atlas-tower.html`) shows `portal-in-1` and `portal-out-1`; P25's spec PR renames them.
- P00's macOS `platforms` evidence is still open: the run on #7 got no GitHub runner (macOS
  capacity). The weekly Monday run, or a re-run, gives it.
- Comments on text sections (decision 0002, #15, phase P19): before P19 starts, Jann settles the
  wording for G7 and CE-23.1 proposed in 0002 §10, which P19's agent tools need. P02 builds the
  UTF-16 positions 0002 asks for.
- Three sentences of the P10 corpus read like lore claims (B-901's link text, "the Cutter is
  near", "the operator-plate names a Listener"). P02's fixture copies reword them neutrally until
  Jann, or a session that can read Confluence, confirms they are canon.
- tower-of-theos#3 (P20, merged) locks `markdown-it-py` 4.3.0 in `tools/repo_checks`, which was
  released the day it was locked. Pinning an older 4.x is a one-line follow-up if Jann prefers a
  cool-down.

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
- Decision 0003 (map generator in Rust, merged in #24): the readable copies of the umbrella spec
  (version 3), the architecture, the data model and the generator protocol; a decision page for
  0003; and Confluence Decision 221741058 (the engine package format), which `docs/map-package.md`
  replaces (Jann confirmed on 10 Nov 2026 that it fixes nothing different).
- Decision 0004 (map generation research, merged in #31): the readable copies of the umbrella
  spec (version 4) and the generator protocol, and a decision page for 0004.
- The P05 split (merged in #34): the readable copies of the umbrella spec (version 6) and of the
  release pipeline (`docs/release.md`).
- The `theos_maps` skill in Jann's other Claude account: once P09 is merged, point it at the
  editor's map generator (`theos gen run` in a shell, or `gen_run` over MCP) or retire it; its
  recipes move into the content project in P23.
- CE-75: move the style bible (`style.yaml` v2 and its reference crops) from Drive into `art/`
  style records, after P20.
- Jira: slices S8 and S9 have no epics yet (create only on Jann's explicit yes).
