---
title: Tyrus MCP
---

# Tyrus MCP

The Tyrus MCP server lets compatible AI clients search the official documentation and check standalone Tyrus source with the compiler's shared analysis. Installation is automatic for supported clients detected on your machine.

## Install

Run one command after installing Tyrus:

```bash
tyrus mcp install
```

The command downloads and verifies a bounded documentation cache, checks the local MCP server handshake, then adds a `tyrus` server entry to detected client configuration files. It supports Claude Desktop, Cursor, Zed, and VS Code Copilot user configuration. Existing settings are preserved, existing `tyrus` entries are not overwritten, and changed configuration files receive a backup before the first edit.

Restart or reload each configured client. Then ask your assistant about Tyrus. The client starts `tyrus mcp serve` in the background; you do not normally run that command yourself. If no supported client is detected, install one and run `tyrus mcp install` again, or follow the manual setup below.

## Commands

| Command | Purpose |
|---------|---------|
| `tyrus mcp install` | Download the documentation cache, configure detected clients, and check the server handshake. |
| `tyrus mcp update` | Refresh the cache and update Tyrus-managed client entries. |
| `tyrus mcp status` | Show cache versions, detected client configuration, and conflicts. |
| `tyrus mcp uninstall` | Remove unchanged Tyrus-managed entries and the local documentation cache. |
| `tyrus mcp serve` | Internal stdio server command started by a configured MCP client. |

## Available MCP features

| Feature | Description |
|---------|-------------|
| `search_docs` | Search indexed headings and return ranked snippets and official page links. |
| `get_page` | Read a complete page listed by the documentation index. |
| `check_source` | Parse, type-check, and borrow-check up to 128 KiB of standalone source in memory. |
| `tyrus://docs/index` | Read the documentation index. |
| `tyrus://docs/{path}` | Read one indexed Markdown page. |
| `tyrus://llms.txt` | Read the compact documentation index. |
| `tyrus://llms-full.txt` | Explicitly request the full documentation bundle. |
| `generate_tyrus_code` | Prompt guidance to find current documentation before generating code. |
| `fix_tyrus_code` | Prompt guidance to check and correct standalone code. |

`search_docs` accepts a query of up to 512 UTF-8 bytes and returns at most 20 results. `get_page` returns at most 1 MiB per page; the full documentation resource is limited to a 32 MiB downloaded bundle. `check_source` accepts up to 128 KiB and stops analysis after five seconds.

`check_source` returns stable diagnostic codes, severities, messages, byte ranges, and one-based line and column positions. It does not execute code, create files, read a project, load environment variables, or resolve project-relative imports. Native module imports are available when the compiler has standalone declarations for them. The analysis is not a security sandbox. Clients and models decide when to call MCP tools, so the server cannot guarantee that an assistant checks code or consults documentation.

## Manual setup

Use the absolute path to your Tyrus executable in the `command` field. Keep other settings in the file. Cursor, Claude Desktop, and Copilot use an `mcpServers` object; Zed uses `context_servers`.

| Client | User configuration location | Root key |
|--------|-----------------------------|----------|
| Claude Desktop | macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`; Windows: `%APPDATA%/Claude/claude_desktop_config.json`; Linux: `${XDG_CONFIG_HOME:-~/.config}/Claude/claude_desktop_config.json` | `mcpServers` |
| Cursor | `~/.cursor/mcp.json` | `mcpServers` |
| Zed | macOS: `~/Library/Application Support/Zed/settings.json`; Windows: `%APPDATA%/Zed/settings.json`; Linux: `${XDG_CONFIG_HOME:-~/.config}/zed/settings.json` | `context_servers` |
| VS Code Copilot | `${COPILOT_HOME:-~/.copilot}/mcp-config.json` | `mcpServers` |

```json
{
  "mcpServers": {
    "tyrus": {
      "command": "/absolute/path/to/tyrus",
      "args": ["mcp", "serve"]
    }
  }
}
```

For Zed, use the same `tyrus` entry under `context_servers` in its settings file. Configuration file locations can vary by operating system, profile, or client version. Prefer each client's MCP settings UI when its configuration path differs from the default. See the official setup guides for [Claude Desktop](https://modelcontextprotocol.io/quickstart/user), [Cursor](https://docs.cursor.com/context/model-context-protocol), [Zed](https://zed.dev/docs/ai/mcp), and [VS Code Copilot](https://code.visualstudio.com/docs/agent-customization/mcp-servers).

## Conflicts and troubleshooting

If a client already has a `tyrus` entry that this installer does not manage, the installer leaves it unchanged and reports a conflict. Review that entry, change or remove it yourself, then run `tyrus mcp install` again. If the configuration file is invalid or cannot be safely preserved, use the client's settings interface or add the entry manually.

Run `tyrus mcp status` to inspect the cache and client entries. Run `tyrus mcp update` to refresh stale documentation. Run `tyrus mcp uninstall` to remove entries that still match the values Tyrus recorded; entries edited after installation are preserved and must be removed manually. Restart the client after changing its MCP configuration.
