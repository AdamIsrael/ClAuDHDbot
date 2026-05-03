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


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
