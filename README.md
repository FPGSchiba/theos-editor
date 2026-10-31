# theos-editor

The Theos Editor — the content studio for Tower of Theos. It edits lore, world, people, story,
concepts and maps as Markdown records in git, checks them against canon on every change, and lets
people (desktop app) and Claude (MCP) work through the same validated commands. Engine support
comes later as an optional part.

## Where to start

| Document | What it holds |
|---|---|
| `docs/specs/00-umbrella.md` | The build: goal, global constraints, phases and acceptance criteria |
| `docs/architecture.md` | Crates, command layer, git, sessions, review, MCP, settings |
| `docs/specs/data-model.md` | The content data model (schema v1) |
| `docs/generators.md` | How maps, concept art and future generators plug in |
| `docs/gate.md` | What a change must pass to reach `main` |
| `docs/status.md` | Where the build stands |
| `docs/decisions/` | Decision log |
| `docs/ui/` | Reference screens |
| `design/theos-ui/` | The Theos UI design system |
| `CLAUDE.md` | How Claude works in this repo |

Related repos: `tower-of-theos` (content project and map toolset), `Theos-Engine` (engine).
