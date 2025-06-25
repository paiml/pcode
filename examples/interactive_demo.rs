use pcode::{
    chat::InteractiveChat,
    tools::{
        file::{FileReadTool, FileWriteTool},
        llm::{LlmTool, TokenEstimateTool},
        process::ProcessTool,
        ToolRegistry,
    },
};
use tokio;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("pcode=info")
        .init();

    // Create tool registry
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FileReadTool));
    registry.register(Box::new(FileWriteTool));
    registry.register(Box::new(ProcessTool));
    registry.register(Box::new(LlmTool::new()));
    registry.register(Box::new(TokenEstimateTool));

    // Run interactive chat
    let mut chat = InteractiveChat::new(registry);
    chat.run().await?;

    Ok(())
}
