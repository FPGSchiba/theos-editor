# theos-vcs

Library. Wrapper over the git command line: status, word diff, history with `--follow`, commit
with trailers, worktrees, merge, pull and push.

After phase P05 the crate provides the runner and the reader of the staged snapshot. The rest
arrives in later phases.

## What P05 provides

| Item | What it does |
|---|---|
| `Git` | Finds the `git` program (`Git::find`) and runs it without a shell (`run`). `with_env` sets the environment of every command, which tests use to isolate git. `find` resolves git's variables once against the current folder, assuming theos runs where git started it (a hook, which git starts in the top, or a user's shell), so a command in a subfolder sees the same repository: a relative `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR` or `GIT_OBJECT_DIRECTORY` is joined to the current folder (git hands hooks `GIT_WORK_TREE=.` or `GIT_DIR=.git`), and when `GIT_DIR` is set and `GIT_WORK_TREE` is not, as for the hooks of a linked work tree or a `--separate-git-dir` repository, `GIT_WORK_TREE` is set to the current folder, git's rule (the variable outranks `core.worktree`, which in a hook names that same folder). `GIT_INDEX_FILE` and `GIT_ALTERNATE_OBJECT_DIRECTORIES` stay as they are. `GitError` is `#[non_exhaustive]` |
| `repo` | `top` (the repository's top folder), `place` (a folder's path inside it), `flag` (a boolean setting, false when unset), `hooks_dir` (honours `core.hooksPath`) and `common_dir` |
| `Folding` | How the repository compares paths: case-folded under `core.ignorecase`, in Unicode NFC under `core.precomposeunicode`, byte for byte otherwise. `same` compares two names, `below` strips a folder's place from a path |
| `staged` | `read` returns the `Staged` snapshot: the staged changes with their `Status`, the tracked paths with their staged objects, and the staged bytes of edited files. git's index decides the spelling: the folder's `place` and every path are spelled as git spells them, and a folder spelled otherwise on disk still matches under the `Folding`. An intent-to-add entry (`git add -N`) is left out, as a commit leaves it out, also when its file is gone (a second read of the staged changes with `--ita-visible-in-index` finds these entries). `blobs` reads the bytes of any staged objects in one `cat-file --batch`. It reads git's `-z`, `ls-files -s` and `cat-file --batch` formats; `is_regular` and `regular_mode` tell a regular file from a link or submodule |

## Who uses what

| Phase | Uses |
|---|---|
| P05 | `Git`, `repo` and `staged` for `theos check --staged` and `theos hook install` |
| P46 | Adds git's version check to `Git::find` (with `TooOld` and a cancel variant in `GitError`) and the reads: status, word diff, history |
| P47 | `staged::read`, `staged::blobs` and `Staged` for the commit, which then adds commit, worktrees, merge, pull and push |
