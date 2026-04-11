# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

claudhdbot — an AI-powered personal assistant Discord bot. Rust edition 2024.

## Build & Run Commands

- **Build:** `cargo build`
- **Run:** `cargo run` (requires `$HOME/.config/claudhdbot/default.toml` + env vars, see `.env.example` and `config/*.example`)
- **Test all:** `cargo test`
- **Test single:** `cargo test <test_name>`
- **Lint:** `cargo clippy`
- **Format:** `cargo fmt`
- **Check (fast compile check):** `cargo check`

## Architecture

Four layers:

- **bot/** — Discord frontend via poise/serenity. Commands in `bot/commands/`. Shared state in `Data` struct (db pool, llm provider, mcp manager, config). Owner-only gate rejects non-owner commands.
- **llm/** — `LlmProvider` trait with `chat(messages, tools) -> LlmResponse`. Providers: Claude (Anthropic API via reqwest), Ollama. LlmResponse is either Text or ToolUse (for MCP tool calls).
- **mcp/** — `McpManager` connects to multiple MCP servers at startup via rmcp. Tools are namespaced as `server.tool`. `McpClient` wraps a single server connection (stdio transport).
- **db/** — SQLite via sqlx with WAL mode. Migrations in `migrations/`. Runtime queries (not compile-time checked).

### Key flow: `/ask` command

User question → LLM with MCP tool definitions → LLM may return tool_use → execute via McpManager → feed results back → loop (max 10 iterations) → final text to Discord.

## Configuration

- `$HOME/.config/claudhdbot/default.toml` — non-secret defaults
- `$HOME/.config/claudhdbot/mcp_servers.toml` — MCP server definitions
- `config/*.toml.example` — checked-in templates to copy into the config dir
- `CLAUDHD_CONFIG_DIR` env var overrides the config directory location
- Environment variables prefixed `CLAUDHD_` with `__` as separator (e.g., `CLAUDHD_DISCORD__TOKEN`)
- Loaded via figment: TOML files merged, then env vars override
