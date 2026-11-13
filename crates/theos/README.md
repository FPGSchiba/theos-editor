# theos

Binary. The command line of a content project. It only parses arguments and prints; every decision
lives in `theos-studio`, so the editor and MCP get the same results.

## Commands

```text
theos check     [<folder>] [--staged] [--binaries <folder>] [--json]
theos ledger    [<folder>] [--json]
theos formalism [<folder>] [--json]
theos schema    [<kind>] [--project <folder>] [--json]
theos new       <kind> [<id>] [--title <text>] [--set <field>=<value>]... [--project <folder>] [--json]
theos hook install [<folder>] [--force] [--json]
```

- `check` prints every finding of the project, with the severities the project configures. With
  `--binaries <folder>`, or a `binaries_root` in the user settings, it also checks the binaries.
  `--staged` checks the snapshot that git would commit, not the working tree; the hook runs it.
- `ledger` prints the reveal ledger of every truth.
- `formalism` regenerates the Shared Formalism mirror and prints each path it wrote.
- `schema` lists every kind and rule; `schema <kind>` describes one kind in full. It needs no
  project.
- `new` makes a record from the template of its kind, in the folder where the kind belongs. It
  writes nothing when the record would introduce an error.
- `hook install` writes git's `pre-commit` hook, which runs `theos check --staged`. A hook that is
  not ours is refused unless `--force` keeps it as `pre-commit.before-theos`.

A `<folder>` or `--project` given is the project root and must hold `theos.toml` (else exit 2).
Without one, the project is the nearest folder at or above the current one that holds
`theos.toml`; `schema` then also runs outside a project, without the configured severities.

## Output

Text is the default. Stdout carries only the result; notes and the summary (`2 errors, 1 warning`)
go to stderr, errors as `error: <message>`. `--json` prints one pretty-printed JSON document on
stdout instead; the summary is part of it.

A finding is one line:

```text
path:line:column: severity rule: message
path: severity rule: message
```

The second form is for a finding without a position. `path` is relative to the project and uses
`/`; line and column start at 1, and the column counts Unicode scalar values. P49's problem
matcher reads these lines, so the format does not change without it.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | The command ran and found no error. Warnings are allowed. |
| 1 | `check` ran and found at least one error. No other command uses 1. |
| 2 | The command could not run: no project, git missing or failing, an I/O error, an invalid flag, a refused `new` or a refused `hook install`. |

## Tests

`tests/cli/` runs the binary against the fixture project in `../theos-studio/tests/fixtures/`,
including commits through the installed hook. `tests/smoke.rs` runs `theos` without arguments.
