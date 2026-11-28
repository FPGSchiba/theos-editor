# Decision log

Decisions taken by Jann. Readable copies and background live in Confluence (TOT). New decisions
are added here as numbered records (`NNNN-<slug>.md`) in a PR that waits for Jann.

| Date | Decision |
|---|---|
| 2026-10-30 | The content editor comes before the remaining engine gates. |
| 2026-10-30 | Claude builds the editor; Jann gives architectural guidance and builds the engine. |
| 2026-10-30 | Three repos: Theos-Engine (library), theos-editor (editor), tower-of-theos (game and content). |
| 2026-10-30 | Records are Markdown with restricted YAML front matter, one per file. |
| 2026-10-30 | The 12 lore pages move to tower-of-theos; system canon stays in Confluence; Shared Formalism is mirrored read-only. |
| 2026-10-30 | Every truth has a reveal target; partly revealed topics such as the false Seal are split into atomic truths. |
| 2026-10-30 | No engine dependency without the `engine` feature. |
| 2026-10-30 | Git through the command line; one worktree per agent session; repo lock; session registry. |
| 2026-10-30 | Review entries derived from agent branches; shared review payload with Content and Engine variants. |
| 2026-10-30 | MCP over stdio, underscore tool names, propose-only. |
| 2026-10-30 | Internal registry of editor parts; public trait with the engine part. |
| 2026-10-30 | Layered TOML settings; secrets in the OS credential store. |
| 2026-10-30 | Binaries in a folder outside git; each user keeps it in sync. |
| 2026-10-31 | Data model schema v1 accepted, with notes on concepts and maps and a generator-neutral model for generated versions. |
| 2026-10-31 | Generators are reached only through the generator protocol (`docs/generators.md`), so the map renderer can change or move into the engine. |
| 2026-10-31 | Specs in this repo are binding for the build; Confluence holds readable copies. |
| 2026-10-31 | Umbrella spec with phases; each phase runs spec → plan → subagent-driven build with superpowers. |
| 2026-10-31 | The final whole-branch review loops with fresh reviewers until clean (cap 5 rounds). Replaced by [0005](0005-judged-review-loop.md). |
| 2026-10-31 | Code PRs merge automatically on green CI; documentation and gate changes wait for Jann; enforced by CLAUDE.md under a shared GitHub identity. |
| 2026-10-31 | UI references are standalone HTML built only from the design system; PNGs are previews rendered from it. |
| 2026-10-31 | Conventional Commits for every commit and PR title; branch names per `CONTRIBUTING.md`. |
| 2026-10-31 | One SemVer product version for all crates; tags `vX.Y.Z` only from the release pipeline; milestones 0.1 (S2) to 1.0 (S7). |
| 2026-10-31 | Releases through release-plz (release PR merged by Jann, one tag) and a tag-triggered workflow for Windows and Linux artifacts with checksums and provenance. |
| 2026-10-31 | macOS is a supported platform: universal builds in every release, tested weekly and before each release (not per PR); ad-hoc signed until an Apple Developer ID is added. |
| 2026-11-04 | The manuscript editor (P12) is built on CodeMirror 6 as a live preview over the Markdown source: [0001](0001-prose-editor.md). |
| 2026-11-05 | Comments on text sections live beside the content, one TOML file per record under `.theos/comments/`, anchored by a text quote; agents reply with comments and change records only by proposals; built in phase P19: [0002](0002-comments-on-text.md). |
| 2026-11-10 | Maps are made by a Rust library in this repo (`theos-maps`, with `theos-map-format`), run in-process by the editor and reusable by the engine; canon floors are baked into `theos-map` 1.0 packages, the same bytes on every platform, and shipped, never regenerated in the game; stacked levels for indoor floors; phases P08, P29, P30: [0003](0003-map-generator-in-rust.md). |
| 2026-11-12 | The map generation research enters P08 and the umbrella (version 4): corrected fluvial law and citations; place types `chime`, `gate-up`, `gate-down`; a `requires` list in map packages; no size limit, a memory budget; P29 and the new P34 at the state of the art with a shared room graph; P30 split from the Atlas half (P45); new P31 Floor frame and places, P32 Recipe families, P33 Map metrics and batch tool; the rest in a research track: [0004](0004-map-generation-research.md). |
| 2026-11-28 | The final review loop judges every finding (fix it, or rule it not worth fixing); after 5 rounds a last fix wave and a targeted review end it as best effort, and the PR stays ready with its open findings listed: [0005](0005-judged-review-loop.md). |
| 2026-11-28 | Review fixes inside a PR that already touches protected paths may change protected paths without asking Jann first; the PR lists them and its approval covers them: [0006](0006-review-fixes-in-a-protected-pr.md). |
