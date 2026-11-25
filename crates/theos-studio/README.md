# theos-studio

Library. Projects, layered settings, editor-part registry, command layer, agent sessions, review
queue, generators, LLM client and binaries folder.

Today it has:

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

The rest arrives in P07 and later phases.
