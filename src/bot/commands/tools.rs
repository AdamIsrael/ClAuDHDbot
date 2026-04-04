use crate::bot::{Context, Error};

/// List all available MCP tools.
#[poise::command(slash_command, prefix_command)]
pub async fn tools(ctx: Context<'_>) -> Result<(), Error> {
    let all_tools = ctx.data().mcp.list_all_tools().await;

    if all_tools.is_empty() {
        ctx.say("No MCP tools available. Configure servers in `config/mcp_servers.toml`.")
            .await?;
        return Ok(());
    }

    let mut output = format!("**Available tools ({}):**\n", all_tools.len());
    for tool in &all_tools {
        output.push_str(&format!(
            "- `{}` — {}\n",
            tool.qualified_name(),
            tool.description
        ));
    }

    // Truncate for Discord
    if output.len() > 2000 {
        output.truncate(1997);
        output.push_str("...");
    }

    ctx.say(output).await?;
    Ok(())
}
