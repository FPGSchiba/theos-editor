# theos-studio

Library. Projects, layered settings, editor-part registry, command layer, agent sessions, review
queue, generators, LLM client and binaries folder.

Today it has the repo lock (`lock`): an OS file lock on `.theos/cache/lock` that lets one process
at a time change a project, with `.theos/cache/lock.owner` naming the holder. The rest arrives in
phases P06 and P07.
