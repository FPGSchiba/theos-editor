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
- stdout carries only the protocol. Logs go to stderr, filtered by `THEOS_LOG` (default `warn`).
- Exit code 0 when stdin closes, 2 when the transport fails.

Tools: `project_info` (more follow in this phase). Every tool is read-only.

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

## Adding a tool

One module in `src/tools/` with its own `#[tool_router(router = <name>_router)]` block, and one
`+` in `tools::router`. Every tool calls `tools::run`.
