# theos

Binary. The command line: `check`, `ledger`, `new`, `schema`, `review` and `hook install`.

Phase P04 adds the first two commands:

- `theos ledger [<folder>]` prints the reveal ledger of every truth.
- `theos formalism [<folder>]` regenerates the Shared Formalism mirror and prints each path it
  wrote.

Both exit with 1 on any error. P05 adds the rest.
