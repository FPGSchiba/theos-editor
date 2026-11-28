# theos-vcs

Library. Wrapper over the git command line: status, history with `--follow`, commit with
trailers, merge, fetch and push. Worktrees and the approval merge arrive in P07.

After phase P47 the crate provides the runner, with git's version check and cancellable runs, the
reader of the staged snapshot, the reads (a file at a version, the status and a file's history)
and the writes: stage and unstage, commit, merge, fetch, push and housekeeping.

## What P05 provides

| Item | What it does |
|---|---|
| `Git` | Finds the `git` program (`Git::find`) and runs it without a shell (`run`). `with_env` sets the environment of every command, which tests use to isolate git. `find` resolves git's variables once against the current folder, assuming theos runs where git started it (a hook, which git starts in the top, or a user's shell), so a command in a subfolder sees the same repository: a relative `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR` or `GIT_OBJECT_DIRECTORY` is joined to the current folder (git hands hooks `GIT_WORK_TREE=.` or `GIT_DIR=.git`), and when `GIT_DIR` is set and `GIT_WORK_TREE` is not, as for the hooks of a linked work tree or a `--separate-git-dir` repository, `GIT_WORK_TREE` is set to the current folder, git's rule (the variable outranks `core.worktree`, which in a hook names that same folder). `GIT_INDEX_FILE` and `GIT_ALTERNATE_OBJECT_DIRECTORIES` stay as they are. `GitError` is `#[non_exhaustive]` |
| `repo` | `top` (the repository's top folder), `place` (a folder's path inside it), `flag` (a boolean setting, false when unset), `hooks_dir` (honours `core.hooksPath`) and `common_dir` |
| `Folding` | How the repository compares paths: case-folded under `core.ignorecase`, in Unicode NFC under `core.precomposeunicode`, byte for byte otherwise. `same` compares two names, `below` strips a folder's place from a path |
| `staged` | `read` returns the `Staged` snapshot: the staged changes with their `Status`, the tracked paths with their staged objects, and the staged bytes of edited files. git's index decides the spelling: the folder's `place` and every path are spelled as git spells them, and a folder spelled otherwise on disk still matches under the `Folding`. An intent-to-add entry (`git add -N`) is left out, as a commit leaves it out, also when its file is gone (a second read of the staged changes with `--ita-visible-in-index` finds these entries). `blobs` reads the bytes of any staged objects in one `cat-file --batch`. It reads git's `-z`, `ls-files -s` and `cat-file --batch` formats; `is_regular` and `regular_mode` tell a regular file from a link or submodule |

## What P46 provides

| Item | What it does |
|---|---|
| `Git::find`, `GitVersion` | `find` runs `git --version` once and refuses a git older than `GitVersion::MINIMUM` (2.38) with `GitError::TooOld`, which names the version found, the program's path and the minimum; `Git::version` is the version read. `GitVersion::parse` takes the first three numbers after `git version ` (`2.56.1.windows.1` is 2.56.1, `2.38.0.rc1` is 2.38.0) |
| `Cancel`, `Git::run_cancellable` | A flag shared between threads. `run_cancellable` checks it every few milliseconds while git runs and stops git with every process it started (Windows: a job object; Unix: a process group and `killpg`), then answers `GitError::Cancelled`. `run` is `run_cancellable` with a flag never set: every git command takes the same spawn path |
| `read` | `read(git, top, Version, path)`: a file's bytes at `Head`, `Index`, any `Rev` or the `WorkTree`, as `Content::File`, `Absent` (also before the first commit, and an intent-to-add entry in the index) or `NotAFile` (a link, a submodule, a folder). Paths are literal, never patterns |
| `status` | `status(git, top, cancel)`: the `Branch` (name or detached, `Upstream` with ahead and behind) and one `Entry` per path: `Changed` with a staged and an unstaged `Change`, `Renamed` (staged, or unstaged renames paired through a temporary index that is always removed), `Conflicted` with its `Conflict`, `Untracked`. The real index is never written, and no hook of the repository runs: the `add --intent-to-add` on the temporary index gets `core.hooksPath` pointed at an empty folder (hook files) and `hook.<name>.enabled=false` for every hook the configuration names (`hook.<name>.event`, git 2.54 and later; the configuration is read only when there are untracked files) |
| `history` | `history(git, top, path, Page { skip, max }, cancel)`: the `Commit`s of a file, newest first, following renames (`--follow`, git's default threshold), each with its id, `Person` after `.mailmap`, time and zone, subject, `Trailer`s, its `Change` and the file's `path` and `from` in that commit. A path never committed and a repository before its first commit have an empty history. Merges as `history`'s docs describe |
| `index` | `paths(git, top, cancel)`: every path of git's index in git's spelling, each once (`ls-files -z`, cancellable). No pathspec, so a folder spelled otherwise on disk is matched by the caller under the `Folding` |
| `repo::first_parent` | The first parent of a commit, `None` for a root commit: what a commit's diff compares with |
| Read options | Every read passes `--no-optional-locks`, `--literal-pathspecs` and the `-c` settings that keep a user's colour, signatures, path quoting and output encoding out of git's answers, and show the root commit (`log.showRoot=true`) |

## What P47 provides

Every write starts with the same options (`write_args`): literal pathspecs, no colour, no path
quoting, no automatic `gc` or maintenance. User settings such as `pull.rebase` or `commit.verbose`
do not change a result (spec §5.12).

| Item | What it does |
|---|---|
| `stage::stage`, `stage::unstage` | Add or remove paths in the index. The paths go to git NUL-separated on stdin, so their number and spelling cannot overflow or change a command line |
| `commit::commit`, `Trailer` | Commits the index with a message and trailers, written by git itself with its hooks running (theos never skips them). Returns the new `CommitId`. `CommitRefused` says what was wrong with the message or a trailer, or that git failed |
| `CommitId` | The id of a commit as git spells it |
| `merge::merge` | Merges an upstream into the current branch and leaves a clean merge staged, uncommitted, so the caller can check it. `Merged` is `UpToDate`, `FastForward`, `Staged` or `Conflicts`. `merge_state` reads a pending merge, `conclude` commits it, `abort_merge` drops it, `merge_base` and `changed_since` find what each side changed |
| `remote::fetch` | Fetches a remote, cancellable. theos never reads or stores a credential: git asks the user's credential helper, and there is no terminal prompt and no askpass program. ssh runs in batch mode unless the user set an ssh command |
| `push::push`, `PushTarget` | Pushes `HEAD` to a branch of a remote, optionally with `--set-upstream`. `Pushed` is `UpToDate` or `Updated`; `PushError::Rejected` carries a `Rejection` (`NotFastForward`, `RemoteRefused`, `Other`) and git's message whole |
| `housekeeping::housekeeping` | Runs `git maintenance run --auto` in the foreground, cancellable |
| `read::checkout_bytes` | The bytes a checkout would write for a path in the index (filters applied), for an undoable discard |
| `repo::head`, `value`, `remotes` | The current commit (`None` before the first), a setting's value (`None` when unset) and the remotes' names |
| `Git::output`, `Git::output_without`, `Output` | Run git and get stdout and stderr whether it succeeded or failed; `output_without` also removes variables from git's environment |
| `GitError::IndexLocked` | Another git holds the index (`index.lock` exists). theos never deletes that file |

## Who uses what

| Phase | Uses |
|---|---|
| P05 | `Git`, `repo` and `staged` for `theos check --staged` and `theos hook install` |
| P46 | Adds git's version check to `Git::find` (with `TooOld` and `Cancelled` in `GitError`) and the reads: `read`, `status`, `history`; `theos-studio` builds `changes`, `diff` and `history` by record on them, `theos-content` the word and field diffs |
| P47 | The writes above. `theos-studio` builds `git_write` on them: `stage_records` and `unstage_records` (both paths of a rename), `commit` after the staged check (`staged::read`, `staged::blobs`), `pull` (`fetch`, `merge`, a check of the merged result), `abort_merge`, `push`, and the discard (`read::checkout_bytes`, only paths git gave). `Cancel` and `run_cancellable` stop fetch and push with every process git started |
| P07 | Worktrees and the approval merge (new in P07), with `commit` and `Trailer` for agent commits and `Approved-by:` on approvals, `Git::output` and `IndexLocked`. `theos-content`'s `diff::record` and `diff::words` for `theos review show` (CE-25.2), with `theos-studio`'s `Side::Between` for an agent branch against `main`. The `Approved-by:` trailer sits on the approval's merge commit, which a file's history need not list: P07 or P13 finds the merge that brought a commit in |
| P12 | `theos-content`'s `diff` to compare an unsaved edit with the saved record |
| P13 | `theos-studio`'s `changes`, `diff` and `history` for the Changes screen (`docs/ui/screens/08-git.html`); `Branch.upstream` for ahead and behind; `Conflicted` entries for CE-36.3; CE-36.2 is `history`. The writes: `stage_records`, `unstage_records`, discard, `commit` with its warnings (CE-36.1), `pull` with `Conflicts`, `merge_state`, `abort_merge`, `push` (`Rejection::NotFastForward` for "Pull first") and `housekeeping` |
| P11, P48 | `Git::find` and its messages when they start git |
