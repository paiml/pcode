use anyhow::Result;
use clap::Parser;
use pcode::{
    chat::InteractiveChat,
    config::Config,
    mcp::McpProtocol,
    runtime::Runtime,
    security::{SecurityContext, SecurityPolicy},
    tools::{
        bash::BashTool,
        dev_cli::DevCliTool,
        file::{FileReadTool, FileWriteTool},
        llm::{LlmTool, TokenEstimateTool},
        pmat::PmatTool,
        process::ProcessTool,
        ToolRegistry,
    },
};
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "pcode")]
#[command(about = "Production-grade AI code agent", long_about = None)]
struct Args {
    #[arg(short, long, help = "Working directory")]
    workdir: Option<PathBuf>,

    #[arg(short, long, help = "Enable debug logging")]
    debug: bool,

    #[arg(long, help = "Disable security sandbox")]
    no_sandbox: bool,

    #[arg(long, help = "Maximum memory usage in MB", default_value = "512")]
    max_memory: usize,

    #[arg(short, long, help = "Run in interactive mode")]
    interactive: bool,

    #[arg(short, long, help = "Execute a command and exit")]
    command: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.debug {
        EnvFilter::from_default_env()
            .add_directive("pcode=debug".parse()?)
            .add_directive("info".parse()?)
    } else {
        EnvFilter::from_default_env()
            .add_directive("pcode=info".parse()?)
            .add_directive("warn".parse()?)
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    info!("Starting pcode v{}", env!("CARGO_PKG_VERSION"));

    // Create runtime
    let runtime = Runtime::new()?;

    runtime.block_on(async_main(args))
}

async fn execute_single_command(registry: ToolRegistry, command: &str) -> Result<()> {
    use pcode::tools::ToolRequest;
    use serde_json::json;
    
    // Parse command - check if it's a tool command
    if command.starts_with('/') {
        let parts: Vec<&str> = command[1..].splitn(2, ' ').collect();
        if parts.is_empty() {
            anyhow::bail!("Invalid command");
        }

        let tool_name = parts[0];
        let params_str = parts.get(1).unwrap_or(&"{}");

        // Parse parameters based on tool
        let params = match tool_name {
            "file_read" => json!({ "path": params_str }),
            "file_write" => {
                let parts: Vec<&str> = params_str.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    json!({ "path": parts[0], "content": parts[1] })
                } else {
                    anyhow::bail!("Usage: /file_write <path> <content>");
                }
            }
            "process" => {
                let parts: Vec<&str> = params_str.split_whitespace().collect();
                if parts.is_empty() {
                    anyhow::bail!("Usage: /process <command> [args...]");
                }
                json!({ "command": parts[0], "args": if parts.len() > 1 { Some(parts[1..].to_vec()) } else { None } })
            }
            "llm" => json!({ "prompt": params_str }),
            "token_estimate" => json!({ "text": params_str }),
            "pmat" => {
                let parts: Vec<&str> = params_str.split_whitespace().collect();
                if parts.len() < 2 {
                    anyhow::bail!("Usage: /pmat <command> <path>");
                }
                json!({ "command": parts[0], "path": parts[1] })
            }
            "bash" => json!({ "command": params_str }),
            "dev_cli" => {
                let parts: Vec<&str> = params_str.split_whitespace().collect();
                if parts.is_empty() {
                    anyhow::bail!("Usage: /dev_cli <tool> [args...]");
                }
                json!({ "tool": parts[0], "args": parts[1..].to_vec() })
            }
            _ => {
                anyhow::bail!("Unknown tool: {}", tool_name);
            }
        };

        // Execute tool
        let request = ToolRequest {
            tool: tool_name.to_string(),
            params,
        };

        let response = registry.execute(request).await;

        if response.success {
            if let Some(result) = response.result {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        } else {
            anyhow::bail!("Error: {}", response.error.unwrap_or_else(|| "Unknown error".to_string()));
        }
    } else {
        // Natural language command - use LLM if available
        let config = Config::from_env();
        if config.has_api_key() {
            let request = ToolRequest {
                tool: "llm".to_string(),
                params: json!({
                    "prompt": command,
                    "max_tokens": 800
                }),
            };

            let response = registry.execute(request).await;
            if response.success {
                if let Some(result) = response.result {
                    if let Some(text) = result.get("response").and_then(|v| v.as_str()) {
                        println!("{}", text);
                    }
                }
            } else {
                anyhow::bail!("LLM error: {}", response.error.unwrap_or_else(|| "Unknown error".to_string()));
            }
        } else {
            println!("No AI Studio API key found. Use tool commands starting with '/' or set AI_STUDIO_API_KEY.");
        }
    }
    
    Ok(())
}

async fn async_main(args: Args) -> Result<()> {
    // Set up security context
    if !args.no_sandbox {
        let policy = SecurityPolicy {
            allowed_paths: vec![args.workdir.clone().unwrap_or_else(|| PathBuf::from("."))],
            allow_network: false,
            allow_process_spawn: true,
            max_memory_mb: args.max_memory,
        };

        match SecurityContext::new(policy) {
            Ok(_) => info!("Security sandbox initialized"),
            Err(e) => {
                error!("Failed to initialize security sandbox: {}", e);
                if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
                    return Err(anyhow::anyhow!("Security sandbox required"));
                }
            }
        }
    } else {
        info!("Running without security sandbox");
    }

    // Initialize tool registry
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FileReadTool));
    registry.register(Box::new(FileWriteTool));
    registry.register(Box::new(ProcessTool));
    registry.register(Box::new(LlmTool::new()));
    registry.register(Box::new(TokenEstimateTool));
    registry.register(Box::new(PmatTool::new()));
    registry.register(Box::new(BashTool::new()));
    registry.register(Box::new(DevCliTool::new()));

    info!("Registered {} tools", registry.list_tools().len());

    // Initialize MCP protocol
    let _protocol = McpProtocol::new();

    info!("pcode ready");

    // Check if we're in interactive mode or have a command
    if args.interactive || args.command.is_none() {
        // Run interactive chat
        let mut chat = InteractiveChat::new(registry);
        chat.run().await?;
    } else if let Some(command) = args.command {
        // Execute single command
        info!("Executing command: {}", command);
        execute_single_command(registry, &command).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = Args::parse_from(["pcode", "--debug", "--max-memory", "1024"]);
        assert!(args.debug);
        assert_eq!(args.max_memory, 1024);
    }
}
