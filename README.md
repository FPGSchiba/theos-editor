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
| `docs/map-package.md` | The map package the editor bakes and the game reads |
| `docs/gate.md` | What a change must pass to reach `main` |
| `CONTRIBUTING.md` | Branches, commits, pull requests, versions and tags |
| `docs/release.md` | The release pipeline |
| `docs/status.md` | Where the build stands |
| `docs/decisions/` | Decision log |
| `docs/ui/` | Reference screens (HTML, with PNG previews) |
| `design/theos-ui/` | The Theos UI design system |
| `CLAUDE.md` | How Claude works in this repo |

Related repos: `tower-of-theos` (content project, map recipes, blockout generator), `Theos-Engine`
(engine). The map generator lives here, in `crates/theos-map-format` and `crates/theos-maps`
(decision 0003).

## Building and the gate

You need `git` and `rustup`. The Rust toolchain is pinned in `rust-toolchain.toml`.

```sh
rustup toolchain install          # reads rust-toolchain.toml
cargo xtask tools --install       # the gate tools, at the versions in gate/tools.toml
cargo xtask gate                  # before you say a change is done
```

`cargo xtask tools --install` compiles cargo-nextest, cargo-deny and cargo-llvm-cov. It needs the
network and takes a few minutes, once per pinned version.

`cargo xtask gate` runs every local check of `docs/gate.md` with the same code as CI, in this
order, and prints one table at the end:

| Check | What it checks |
|---|---|
| `tools` | The installed tools have the versions pinned in `gate/tools.toml` |
| `fmt` | `cargo fmt --all --check` |
| `lints` | The lint list, the tool configuration and the attributes in the sources |
| `engine-free` | No engine crate in the tree without the feature `engine` |
| `ac-trace` | Every acceptance criterion of the phase has a tagged test; every tag names one |
| `commit-msg` | Every commit since `main` is a Conventional Commit |
| `clippy` | `cargo clippy` on every crate and target, warnings denied |
| `docs` | `cargo doc` with warnings denied |
| `deny` | `cargo deny check`: licences, advisories, bans and sources |
| `test` | The tests (nextest) and the doc tests |
| `coverage` | Line coverage per crate against its floor and against `main` |

It keeps going after a check fails and exits 1 if any failed. When a tool is not the pinned
version it stops after `tools`, because versions decide the verdicts. `commit-msg` and `coverage`
compare with `origin/main`, so fetch it first.

The gate leaves out the three checks that need a pull request: `pr-title`, `pr-body` and
`approval`. They run in CI.

Each check is also a subcommand of its own name, such as `cargo xtask clippy`.

- `cargo xtask ac-trace --phase PNN` lists the acceptance criteria of phase `PNN` and the tests
  that prove them, instead of the phase of the current branch.
- `cargo xtask github check` compares the rulesets, merge settings and labels of the repository on
  GitHub with `gate/` and reports every difference. It only reads and needs the `gh` command line.
  Changing them (`cargo xtask github apply`) is for Jann to agree to first.
