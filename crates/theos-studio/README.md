# theos-studio

Library. Projects, layered settings, editor-part registry, command layer, agent sessions, review
queue, generators, LLM client and binaries folder.

After phases P05 and P06 the crate provides the workspace, the checks and the JSON views that
`theos` prints and that later phases serve again, the repo lock and the command layer. The rest
arrives in P07 and later phases.

## What P05 provides

| Item | What it does |
|---|---|
| `Workspace` | A project with its kind registry and its index, opened once. `open(root)` and `find(start)` sync the index; `find_root` finds the project above a folder. When another process holds the cache (`IndexError::Contended`), the index lives in memory instead, and `is_in_memory()` says so |
| `StudioError`, `report` | The error of the crate. `report` writes an error with each cause once; `theos` prints it after `error:` and P48 uses it for error results |
| `check` | The findings of the project with configured severities, plus the warnings of the binaries folder (`Binaries`, `BinariesFolder`, `binaries_folder`) |
| `check_staged` | Judges the snapshot git would commit, read through `theos-vcs`, and returns a `StagedCheck`: `blocks()`, `errors()`, `warnings()`. Paths are judged as git's index spells them, also when the disk spells a file or the project folder otherwise (a case-only rename). The working tree, the cache on disk and the repository stay as they are, and no lock is taken |
| `introduced` | The error findings a change introduces beyond the counts a `FindingCounts` allows per path and rule; the verdict of the command layer uses it, so it also refuses a record `create` makes |
| `create` | Makes a new record from the template of its kind and runs P06's `Create` with it, so it is written only when it introduces no error, behind the repo lock like every command |
| `install_hook` | Writes git's `pre-commit` hook, which runs `theos check --staged`, and the same script as `pre-merge-commit` for the merge commit of `git merge`; both files are judged before either is written |
| `user_settings` | Reads where the user settings are and the `binaries_root` they name |
| `schema` | The kinds and rules as data |
| `views` | The JSON shapes of check, ledger, new, schema and hook results; every key is always present, an absent value is `null` |

## What P06 provides

- the repo lock (`lock`): an OS file lock on `.theos/cache/lock` that lets one process at a time
  change a project, with `.theos/cache/lock.owner` naming the holder;
- the `Workspace`: a project with its editor-part registry and index;
- the command layer: `Create`, `Write`, `Edit`, `Patch`, `Delete` and `Rename` plan their
  changes, and `Workspace::run` takes each through one pipeline: the repo lock, a fresh index,
  the base check, the preview, the verdict (a command that introduces an error is rejected), the
  write, the reindex and the route's finish. `Rename` renames an id with its file, its child
  folder, a concept's or map's versions and every link the index knows, in one command. The
  human route keeps the undo journal; P07 adds the agent route behind the same `Route` trait;
- undo and redo: `Workspace::undo` and `Workspace::redo` play a journal entry back through the
  same pipeline, to the exact bytes. `Scope::Newest` takes the newest entry, `Scope::Record` the
  newest entry that changed one file. The journal lives in memory and keeps the last 100
  commands.

## Who uses what

| Phase | Uses |
|---|---|
| P06 | `Workspace` (`open`, `find`, `index_mut`), extended with its own `impl` blocks for the command layer and the repo lock; `introduced`, in its verdict. In turn `create` writes through P06's `Create` |
| P47 | `check_staged`, so a commit blocks on the same findings as the hook; it holds the repo lock while git runs the hook, and the check never takes it |
| P48 | `Workspace`, the `views`, `report` and the contended-cache fallback, to serve the same results over MCP |
| P14 | `user_settings::binaries_root`, grown into the three layers of the settings |
