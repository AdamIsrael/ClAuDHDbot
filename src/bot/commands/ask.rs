use crate::bot::{Context, Error};
use crate::llm::{ChatMessage, LlmResponse, MessageContent, Role};
use crate::mcp::McpManager;

const MAX_TOOL_ITERATIONS: usize = 10;

/// Ask the AI a question (with MCP tools available).
#[poise::command(slash_command, prefix_command)]
pub async fn ask(
    ctx: Context<'_>,
    #[description = "Your question"]
    #[rest]
    question: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();
    let tools = data.mcp.list_all_tools().await;

    let mut messages = vec![
        ChatMessage {
            role: Role::System,
            content: MessageContent::Text(
                "You are ClAuDHDbot, a helpful personal assistant. Be concise and direct."
                    .to_string(),
            ),
        },
        ChatMessage {
            role: Role::User,
            content: MessageContent::Text(question),
        },
    ];

    // Tool use loop
    for _ in 0..MAX_TOOL_ITERATIONS {
        let response = data.llm.chat(&messages, &tools).await?;

        match response {
            LlmResponse::Text(text) => {
                // Truncate to Discord's 2000 char limit
                let reply = if text.len() > 2000 {
                    format!("{}...", &text[..1997])
                } else {
                    text
                };
                ctx.say(reply).await?;
                return Ok(());
            }
            LlmResponse::ToolUse(tool_calls) => {
                // Execute each tool call and feed results back
                for call in &tool_calls {
                    messages.push(ChatMessage {
                        role: Role::Assistant,
                        content: MessageContent::ToolUse {
                            id: call.id.clone(),
                            name: call.name.clone(),
                            input: call.input.clone(),
                        },
                    });

                    let result = execute_tool_call(&data.mcp, &call.name, &call.input).await;

                    let (content, is_error) = match result {
                        Ok(val) => (val["content"].as_str().unwrap_or("").to_string(), false),
                        Err(e) => (format!("Error: {e}"), true),
                    };

                    messages.push(ChatMessage {
                        role: Role::User,
                        content: MessageContent::ToolResult {
                            tool_use_id: call.id.clone(),
                            content,
                            is_error,
                        },
                    });
                }
            }
        }
    }

    ctx.say("Reached maximum tool iterations. Here's what I have so far.")
        .await?;
    Ok(())
}

async fn execute_tool_call(
    mcp: &McpManager,
    tool_name: &str,
    args: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let (server, tool) = McpManager::parse_tool_name(tool_name).ok_or_else(|| {
        anyhow::anyhow!("Invalid tool name format: {tool_name} (expected server.tool)")
    })?;

    mcp.call_tool(server, tool, args.clone()).await
}
