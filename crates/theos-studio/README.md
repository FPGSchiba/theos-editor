# theos-studio

Library. Projects, layered settings, editor-part registry, command layer, agent sessions, review
queue, generators, LLM client and binaries folder.

After phase P05 the crate provides the workspace, the checks and the JSON views that `theos`
prints and that later phases serve again. The rest arrives in P06, P07 and later phases.

## What P05 provides

| Item | What it does |
|---|---|
| `Workspace` | A project with its kind registry and its index, opened once. `open(root)` and `find(start)` sync the index; `find_root` finds the project above a folder. When another process holds the cache (`IndexError::Contended`), the index lives in memory instead, and `is_in_memory()` says so |
| `StudioError`, `report` | The error of the crate. `report` writes an error with each cause once; `theos` prints it after `error:` and P48 uses it for error results |
| `check` | The findings of the project with configured severities, plus the warnings of the binaries folder (`Binaries`, `BinariesFolder`, `binaries_folder`) |
| `check_staged` | Judges the snapshot git would commit, read through `theos-vcs`, and returns a `StagedCheck`: `blocks()`, `errors()`, `warnings()`. Paths are judged as git's index spells them, also when the disk spells a file or the project folder otherwise (a case-only rename). The working tree, the cache on disk and the repository stay as they are, and no lock is taken |
| `introduced` | The error findings a change introduces beyond the counts a `FindingCounts` allows per path and rule; `create` uses it to refuse a record |
| `create` | Makes a new record from the template of its kind, and writes it only when it introduces no error |
| `install_hook` | Writes git's pre-commit hook, which runs `theos check --staged` |
| `user_settings` | Reads where the user settings are and the `binaries_root` they name |
| `schema` | The kinds and rules as data |
| `views` | The JSON shapes of check, ledger, new, schema and hook results; every key is always present, an absent value is `null` |

## Who uses what

| Phase | Uses |
|---|---|
| P06 | `Workspace` (`open`, `find`, `index_mut`), extended with its own `impl` blocks for the command layer and the repo lock; `introduced`; moves `theos new` onto its create command |
| P47 | `check_staged`, so a commit blocks on the same findings as the hook; it holds the repo lock while git runs the hook, and the check never takes it |
| P48 | `Workspace`, the `views`, `report` and the contended-cache fallback, to serve the same results over MCP |
| P14 | `user_settings::binaries_root`, grown into the three layers of the settings |
