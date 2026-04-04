# Specification for ClAuDHDbot

An AI-powered 24/7 personal assistant.

Similar to [openclaw](https://openclaw.ai/), but is meant to limit what credentials and secrets it has access to. The majority of functionality will come from integrations from MCP servers, bespoke and borrowed.

## Tech Stack

- Rust
- Axum (if needed)
- Tokio (asynchronous framework)
- MCP (the ability to interact with MCP servers)
- LLM(s)
	- Claude
	- Ollama
	- OpenAPI-compatible
- SQLite


## Use-cases

### Daily Digest

Every day, send the user a digest with relevant information, such as appointments for the day, task(s) that require attention, maybe the weather forecast.

### Discord

The user will primarily communicate with the bot via Discord private message. The user will use this channel to request information, manage tasks or calendar events, set reminders, etc.

### Tasks

The user is forgetful. Claudhd is not. The user can ask Claudhd to remember tasks that need to be done, along with their priority and context. The user can then list, claim, release, delete or finish tasks.

## MCP Servers

### Github

A Github MCP that provides a way to interact with a Github account.
