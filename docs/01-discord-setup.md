# Discord Bot Setup

## 1. Create a Discord Application

1. Go to https://discord.com/developers/applications
2. Click **New Application**, give it a name (e.g. "ClAuDHDbot")
3. Go to the **Bot** tab in the left sidebar
4. Copy the **Token** — this is your `CLAUDHD_DISCORD__TOKEN`

## 2. Enable Privileged Intents

On the **Bot** tab, scroll to **Privileged Gateway Intents** and enable:

- **Message Content Intent** (required for prefix commands in DMs)

## 3. Invite the Bot to a Server

Even for DM-only usage, you must share at least one server with the bot.

Go to the **OAuth2** tab, then **URL Generator**:

1. Under **Scopes**, select `bot` and `applications.commands`
2. Under **Bot Permissions**, select:
   - Send Messages
   - Use Slash Commands
3. Copy the generated URL and open it in your browser
4. Select a server to add the bot to (create a private server if needed)

## 4. Get Your Owner ID

1. In Discord, go to **Settings → Advanced** and enable **Developer Mode**
2. Right-click your own username and click **Copy User ID**
3. This is your `CLAUDHD_DISCORD__OWNER_ID`

## 5. Configure and Run

Create a `.env` file (see `.env.example`):

```
CLAUDHD_DISCORD__TOKEN=your_bot_token
CLAUDHD_DISCORD__OWNER_ID=your_user_id
CLAUDHD_LLM__PROVIDER=openai
CLAUDHD_LLM__BASE_URL=http://localhost:1234/v1
CLAUDHD_LLM__MODEL=your-model
```

Run:

```
cargo run
```

## 6. Messaging the Bot

Once the bot is running and shows "Bot is ready!":

- **Slash commands**: Type `/ping`, `/task`, `/ask`, or `/tools` in any channel or DM
- **DMs**: Click the bot's name in the server member list and send a direct message
- Slash commands may take up to an hour to register globally after first run. For instant testing, send prefix commands like `~ping` (or use the bot in DMs).

> **Note**: Slash commands register globally on first startup. Discord can take up to an hour to propagate them. If commands don't appear immediately, wait or try restarting Discord.
