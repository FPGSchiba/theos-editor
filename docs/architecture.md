# Architecture — Theos content editor

Status: accepted by Jann on 30 Oct 2026; generators added 31 Oct 2026; atlas, quests and art
pipelines added with umbrella spec version 2; the map generator in Rust added with umbrella spec
version 3 (decision 0003). Binding.
Readable copy for people: Confluence TOT › "Architecture: Theos Editor — content-first".
Engine integration (later) is described on the Confluence pages "Tauri-Based Editor GUI" and
"Hybrid Editor Architecture & IPC"; nothing here may contradict them.

## 1. Goals

| Goal | Consequence |
|---|---|
| Story work first | Lore is writable as files long before the UI exists. |
| No engine dependency | The editor builds and runs without any engine crate. |
| Engine later, without a rewrite | Engine support is one more editor part behind the cargo feature `engine`. |
| Human and agent, one API | UI, CLI and MCP call the same command layer; every write passes the same validation. |
| Plain files, git history | Markdown records in git; indexes and generated views are rebuilt, never committed; binaries stay outside git. |
| Built by Claude, owned by Jann | Small PRs, rustdoc, decision log, strict gate (`docs/gate.md`). |
| Windows first | Primary platform; every PR is checked on Linux; Windows and macOS are tested weekly, on demand and before every release; macOS ships as universal builds. |

## 2. Repositories

| Repo | Holds | Built by |
|---|---|---|
| Theos-Engine | The engine as a library, including `theos-editor-api` (engine side of the editor IPC). | Jann |
| theos-editor (this repo) | Content crates, the map generator (`theos-map-format`, `theos-maps`), `theos` CLI, `theos-mcp`, the Tauri app, `editor-ui/`, gate tooling. | Claude |
| tower-of-theos | The content project now, the game binary later; map recipes and the blockout generator in `tools/`. | Jann (content with Claude through MCP) |

A content project is any folder with a `theos.toml`. It must live on a local disk, not on a
synced or streamed drive.

```text
tower-of-theos/
  theos.toml              manifest (kind, schema version, enabled parts)
  .theos/                 settings.toml (tracked) · sessions/ · cache/ (ignored)
  world/ people/ things/  world and people trees (spawns sit under their place), things
  enemies/                enemy kinds
  lore/                   topics/ truths/ beliefs/ clues/
  story/scripts/          one file per script; beats inside
  story/quests/           one file per quest; steps inside
  concepts/ maps/         parents and their versions (binaries live in the binaries folder);
                          map bodies hold pins and cameras
  art/                    style records
  templates/ dialogue/    compile and prompt templates · kept dialogue lines
  canon/formalism/        E1…E18, generated read-only mirror of Shared Formalism
  tools/                  maps/recipes, theos_blockout (Python generator), comfy/workflows,
                          comfy/pipelines, guides (for agents), import scripts
```

## 3. Crates

| Crate | Type | Role |
|---|---|---|
| `theos-content` | lib | Record model, front-matter patcher, body grammars (beats, voice, notes), kind registry, index, search, effective lenses, validators, lore rules, ledger. Pure filesystem; no async; no UI. |
| `theos-vcs` | lib | Wrapper over the git command line: status, word diff, history with `--follow`, commit with trailers, worktrees, merge, pull, push. |
| `theos-studio` | lib | Projects, layered settings, editor-part registry, command layer, agent sessions, review queue, generators (`docs/generators.md`), LLM client, binaries folder. |
| `theos-map-format` | lib | The map package (`docs/map-package.md`): types, reader and writer. Depends on no other crate of this workspace, so the engine can depend on it alone. |
| `theos-maps` | lib | The map generator (decision 0003): recipes, the stage pipeline, terrain, settlement and interior maps, renders, packages; deterministic on every platform (G20). `theos-studio` runs it in-process. |
| `theos` | bin | CLI: `check`, `ledger`, `new`, `schema`, `review …`, `gen …` (runs generators into a folder, never writes records), `hook install`. |
| `theos-mcp` | bin | MCP server over stdio. |
| `theos-editor` | bin (Tauri 2) | Authoring shell; Tauri commands wrap the command layer; typed IPC bindings generated for the UI. Feature `engine` (off) pulls engine crates from Theos-Engine as git dependencies. |
| `xtask` | bin | Gate tooling (`docs/gate.md`) and generator conformance checks. |
| `editor-ui/` | Svelte 5 + TS | Shell, presets and windows built from `design/theos-ui/`. |

Dependency rule: without the `engine` feature, no crate depends on an engine crate. CI checks it.
The reverse direction is allowed: the engine may depend on `theos-map-format` and `theos-maps`.

## 4. Content model

`docs/specs/data-model.md` is the contract. Principles: one record per file; restricted YAML
edited by a span patcher; links stored once and backlinks derived; clue records are the only
source of the ledger; lenses embedded in holders and resolved people → faction → npc; reveal
targets per truth; system canon stays in Confluence (Shared Formalism mirrored read-only);
generated views never committed; concepts and maps share one generator-neutral model.

## 5. Command layer

```text
command ─► schema check ─► validators (record + neighbours + requires chain) ─► route
   human  → write working tree → reindex → event (undo entry)
   agent  → write in the session worktree → commit on agent/<session> → review entry → event
queries read the index; it rebuilds from files on open and on file change
```

Blocking findings reject the command; warnings come back with it. Undo is per command. An
external change to an open record reloads it if clean, or offers a merge if dirty.

## 6. Editor parts

Codex, Atlas, Changes, Scripts, Graph, Concepts, Maps, Review, MCP — "extensions" in the UI. In v1
an internal registry compiled into the editor: each part registers kinds, rules, commands, MCP
tools, rail presets and settings. A public trait comes with the engine part. A rail preset replaces
the whole dock tree. Content build rail: Codex, Atlas, Graph, Scripts, Concepts, Maps, Review,
Changes, Code, Settings.

- **Atlas** shows the world on its maps: the Tower view (bands, floors, the main quest across
  them), and floor, settlement and interior maps with layers for places, NPCs, spawns, quest
  routes, lore and concepts, filtered by Understanding tier. Clicking a place opens its lore,
  people, quests, concepts and its own map. Placing a record writes a pin line in the map's body
  (`docs/specs/data-model.md` §9.6). Maps stays the generator workbench (runs, seeds, picks).
- **Scripts** also holds quests: a quest is shown as a flow diagram drawn from its steps.
- **Concepts** also starts a concept from a record: family, pipeline, brief, prompt, camera.

## 7. Git

The git command line only. Ctrl+S writes files; committing is a step in Changes. Pre-commit
validation runs on the staged snapshot. Changes grouped by record; prose diffs word-level, front
matter by field; history follows renames. `theos hook install` and CI run `theos check`, so edits
made outside the editor cannot commit a guardrail violation.

## 8. Agent sessions and review

- One worktree per agent session in `.theos/cache/worktrees/<session>` on `agent/<session>`.
- One repo lock (`.theos/cache/lock`) for git writes across the editor, CLI and all `theos-mcp`
  processes.
- Session registry `.theos/sessions/<id>.json`: brief, scope, reserved id block, tool log, pause flag.
- Review entries are derived from agent-branch commits. Approve = merge with an `Approved-by:`
  trailer after re-validating the merged result; waits if the human has uncommitted edits to the
  same records. Significance is a level (low, medium, high); canon truths and `never` truths are high.
- `theos review` offers the same from the CLI. Shared `ReviewPayload` has `Content` and `Engine`
  variants (engine entries keep the JSONL flow of the Tauri GUI page).

## 9. MCP server

- stdio only; several clients mean several processes, coordinated by the lock and the registry.
- Read tools: `project_info`, `codex_search`, `codex_read`, `codex_list`, `codex_graph`,
  `ledger_check`, `review_list`, `gen_catalogue`, `gen_runs`, `atlas_read`, `quest_flow`,
  `brief_compose`, `pipeline_list`.
- Propose tools: `codex_propose`, `script_propose`, `gen_run`, `gen_pick`, `note_add`,
  `note_resolve`, `atlas_place`, `quest_propose`, `pipeline_run`.
- Prompts: the guides in the content project's `tools/guides/*.md` (asset workflow, placing on the
  atlas, writing a quest) are served as MCP prompts and listed by `project_info`, so the way to
  work is written down once, next to the content.
- Never over MCP: approving, pushing, changing settings, editing locked records (canon truths,
  `never` truths, the formalism mirror), locking or unlocking a style.

## 10. Guardrails

| Rule | Guards | Can be lowered? |
|---|---|---|
| never-confirm | Nothing reveals a `never` truth. | No |
| no-overshoot | No clue or beat pushes a truth past its target. | No |
| inworld-vocab | In-world text has no developer words; allowlist entries need a reason. | No |
| reveal-order, requires, three-clue | Order and robustness of reveals. | Error or warning |

LLM drafts run through all lore rules before they are shown. Text of `never` truths is withheld
from every LLM prompt and from every brief for a generator. Quest steps are checked like beats, on
every path through the quest.

## 11. Generators

`docs/generators.md`. Three adapters in v1 (built-in, process, ComfyUI). The map generator is the
crate `theos-maps`, run in-process on worker threads through the built-in adapter (decision 0003):
no Python is needed for maps. It bakes each picked map into a package (`docs/map-package.md`), the
same bytes on every platform (G20). The blockout generator is a Python process generator run
through uv with Python 3.12 in `.theos/cache/venv`. Outputs go to the binaries folder; versions
become records; open notes go into every run request.

The art pipeline, end to end:

```text
record ─► brief (prompt template + lore + effective style, tier-filtered) ─► prompt (agent, AI draft or human)
map ─► blockout generator ─► clay / depth / canny per camera
pipeline stage ─► ComfyUI workflow (slots filled from prompt, picks, style refs, camera renders)
   ─► concept versions ─► pick ─► next stage … ─► canon pick
```

- The blockout generator (`tools/theos_blockout`) builds one clay scene of a place, from a
  settlement or interior map or from a blockout script, so every view of the place shares its
  shapes.
- Prepared workflows and pipelines live in `tools/comfy/`; each workflow states its models and
  peak VRAM, and one stage runs at a time, so the chain fits a 16 GB GPU.
- Asset families (buildings, landscapes, characters, NPCs in a scene, creatures, items and clothes,
  in-world maps) are pipelines; a pick of one concept can feed another through `ref.<kind>`.

## 12. LLM client

One OpenAI-compatible endpoint (local or the cluster), configured by the user, for compile
previews and commit-message suggestions. Output is always an AI-marked draft.

## 13. Settings and projects

Three layers: part default → user (`%APPDATA%\theos\settings.toml` on Windows,
`$XDG_CONFIG_HOME/theos/settings.toml` on Linux, `~/Library/Application Support/theos/settings.toml`
on macOS; holds `binaries_root`) → project (`.theos/settings.toml`, tracked). Format-preserving TOML;
secrets in the OS credential store. Project kinds: `content` now, `game` with the engine part,
`tool` for sandboxes.

## 14. Path to the engine

The engine part adds Layout, AI and Assets presets, scene windows, PIE and game projects through
`theos-editor-api`. The game engine may become a generator (`docs/generators.md`). It reads map
packages with `theos-map-format` and ships the baked package of each canon map; it never
regenerates a canon map (G20). Pins are fractions of a map's frame, so the engine can refine a
baked map at another size or resolution and place everything where it was designed; the engine
part adds tuning fields
(stats, spawn tables, timers, loot) to enemies and spawns and an export of pins, spawns and quests. THEOS-138 and THEOS-140 are covered by this editor's shell and Review.
