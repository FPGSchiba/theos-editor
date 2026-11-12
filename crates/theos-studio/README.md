# theos-studio

Library. Projects, layered settings, editor-part registry, command layer, agent sessions, review
queue, generators, LLM client and binaries folder.

Today it has:

- the repo lock (`lock`): an OS file lock on `.theos/cache/lock` that lets one process at a time
  change a project, with `.theos/cache/lock.owner` naming the holder;
- the `Workspace`: a project with its editor-part registry and index;
- the command layer: `Create`, `Write`, `Edit`, `Patch` and `Delete` plan their changes, and
  `Workspace::run` takes each through one pipeline: the repo lock, a fresh index, the base check,
  the preview, the verdict (a command that introduces an error is rejected), the write, the
  reindex and the route's finish. The human route keeps the undo journal; P07 adds the agent
  route behind the same `Route` trait.

The rest arrives in phases P06 and P07.
