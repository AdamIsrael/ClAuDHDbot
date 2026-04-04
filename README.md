# ClAuDHDbot

An AI-powered personal assistant Discord bot. Communicates via DM, uses MCP servers for integrations, and supports multiple LLM providers.

## Features

- **Discord DM interface** — all commands are DM-only, restricted to the configured owner
- **Task management** — create, list, claim, release, complete, and delete tasks with priorities
- **LLM integration** — ask questions with Claude, Ollama, or any OpenAI-compatible API
- **MCP tool use** — connects to MCP servers at startup; the LLM can call their tools to answer questions
- **Scheduled messages** — cron-based scheduler that DMs you on a schedule (e.g., daily digest)

## Quick Start

### Prerequisites

- Rust 1.85+ (edition 2024)
- A Discord bot token ([setup guide](docs/01-discord-setup.md))
- An LLM provider (local Ollama, OpenAI-compatible API, or Anthropic API key)

### Configure

Copy `.env.example` to `.env` and fill in your values:

```
CLAUDHD_DISCORD__TOKEN=your_discord_bot_token
CLAUDHD_DISCORD__OWNER_ID=your_discord_user_id
CLAUDHD_LLM__PROVIDER=openai
CLAUDHD_LLM__BASE_URL=http://localhost:1234/v1
CLAUDHD_LLM__MODEL=your-model
```

### Run

```
cargo run
```

## Commands

| Command | Description |
|---|---|
| `/ping` | Health check |
| `/ask <question>` | Ask the LLM a question (with MCP tools available) |
| `/task add <title> [priority] [description]` | Create a task |
| `/task list [status]` | List tasks |
| `/task done <id>` | Mark a task complete |
| `/task claim <id>` | Mark a task in progress |
| `/task release <id>` | Mark a task pending |
| `/task delete <id>` | Delete a task |
| `/schedule add <name> <cron> <message>` | Create a scheduled DM |
| `/schedule list` | List scheduled jobs |
| `/schedule remove <name>` | Delete a scheduled job |
| `/schedule enable <name>` | Enable a scheduled job |
| `/schedule disable <name>` | Disable a scheduled job |
| `/tools` | List available MCP tools |

## Configuration

Configuration is loaded from TOML files and environment variables (via [figment](https://docs.rs/figment)):

- `config/default.toml` — non-secret defaults (database URL, LLM model)
- `config/mcp_servers.toml` — MCP server definitions
- `.env` — secrets (loaded via dotenvy)
- Environment variables prefixed `CLAUDHD_` with `__` as nested separator

### MCP Servers

Define MCP servers in `config/mcp_servers.toml`:

```toml
[[mcp.servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/some/path"]
```

Servers are connected at startup via stdio transport. Tools are namespaced as `server.tool` and made available to the LLM.

## Tech Stack

- **Rust** (edition 2024) with **Tokio** async runtime
- **Poise** / **Serenity** — Discord bot framework
- **SQLite** via **sqlx** — task and schedule persistence
- **rmcp** — MCP client (Model Context Protocol)
- **reqwest** — LLM API calls (Claude, Ollama, OpenAI-compatible)
- **tokio-cron-scheduler** — scheduled job execution
- **figment** — hierarchical configuration
