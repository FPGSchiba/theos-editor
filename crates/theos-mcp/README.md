# theos-mcp

Binary. The read-only MCP server of a Theos content project, over stdio.

```text
theos-mcp [--project <folder>]
```

- Without `--project` it uses the nearest folder at or above the current one that holds
  `theos.toml`. With it, the folder must hold `theos.toml`; it is never searched upward.
- A start outside a project does not end the process: every tool call answers with an error
  result that names the reason, and the project opens on the first call after `theos.toml`
  exists.
- stdout carries only the protocol. Logs go to stderr, filtered by `THEOS_LOG` (default `warn`),
  for example `THEOS_LOG=debug`.
- Exit code 0 when stdin closes. Exit code 2 when the server cannot start or the MCP handshake
  fails. After the handshake, rmcp 3.12 treats a failed stdin read as closed input and only
  logs a failed stdout write, so the server then exits with 0.
- `--version` is not offered yet.

## Tools

The server is read-only: it never changes a record. Every tool's parameters are a JSON object,
and an unknown parameter is an error result.

| Tool | Parameters | Answers |
|---|---|---|
| `project_info` | none | folder, schema version, cache, record counts per kind, rule severities, finding counts, guides, tools |
| `codex_search` | `query`, `limit` (1-100, default 20) | the best hits, with where the words match |
| `codex_read` | `id` or `path` (exactly one) | one record: text, links, backlinks, children, findings, effective lens with beliefs, and the reveal ledger of a truth |
| `codex_list` | `kind`, `status`, `folder`, `offset`, `limit` (1-500, default 100) | the records that match all filters, ordered by path |
| `codex_graph` | `id`, `depth` (1-3, default 1), `direction` (`out`, `in`, `both`) | the records around one record, by distance, and the links between them |
| `ledger_check` | `id` (optional: one truth) | the reveal ledger and the findings of the `lore/` rules |

## Marks

Every record a result names carries two marks, next to its `id`:

- `never`: the record is a truth with target `never`. Claude must never confirm it.
- `locked`: the reasons the record cannot be changed over MCP (`canon`, `formalism`, `style`,
  `never`). A locked record is still readable in full.

This holds for search hits, links, backlinks, lens values and beliefs, graph nodes, ledger truths,
clues, places and scripts.

## Prompts

The guides of the project's `tools/guides/*.md` are served as MCP prompts, one per guide, and a
new guide appears without a restart.

## Claude Code

Claude Code starts a project's MCP server in the project folder, so `.mcp.json` needs no
argument:

```json
{
  "mcpServers": {
    "theos": { "command": "theos-mcp" }
  }
}
```

To serve a fixed project, add `"args": ["--project", "<folder>"]`.

## Adding a tool

One module in `src/tools/` with its own `#[tool_router(router = <name>_router)]` block, and one
`+` in `tools::router`. Every tool calls `tools::run`, and its description ends with the shared
`read_only_note!()`.
