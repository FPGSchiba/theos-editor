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
| 2026-10-31 | The final whole-branch review loops with fresh reviewers until clean (cap 5 rounds). |
| 2026-10-31 | Code PRs merge automatically on green CI; documentation and gate changes wait for Jann; enforced by CLAUDE.md under a shared GitHub identity. |
| 2026-10-31 | UI references are standalone HTML built only from the design system; PNGs are previews rendered from it. |
| 2026-10-31 | Conventional Commits for every commit and PR title; branch names per `CONTRIBUTING.md`. |
| 2026-10-31 | One SemVer product version for all crates; tags `vX.Y.Z` only from the release pipeline; milestones 0.1 (S2) to 1.0 (S7). |
| 2026-10-31 | Releases through release-plz (release PR merged by Jann, one tag) and a tag-triggered workflow for Windows and Linux artifacts with checksums and provenance. |
| 2026-10-31 | macOS is a supported platform: universal builds in every release, tested weekly and before each release (not per PR); ad-hoc signed until an Apple Developer ID is added. |
| 2026-11-04 | The manuscript editor (P12) is built on CodeMirror 6 as a live preview over the Markdown source: [0001](0001-prose-editor.md). |
| 2026-11-05 | Comments on text sections live beside the content, one TOML file per record under `.theos/comments/`, anchored by a text quote; agents reply with comments and change records only by proposals; built in phase P19 (proposed): [0002](0002-comments-on-text.md). |
