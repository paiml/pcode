use anyhow::Result;
use clap::Parser;
use pcode::{
    mcp::McpProtocol,
    runtime::Runtime,
    security::{SecurityContext, SecurityPolicy},
    tools::{
        file::{FileReadTool, FileWriteTool},
        llm::{LlmTool, TokenEstimateTool},
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

    info!("Registered {} tools", registry.list_tools().len());

    // Initialize MCP protocol
    let _protocol = McpProtocol::new();

    info!("pcode ready");

    // Main event loop would go here
    // For now, just demonstrate functionality

    let tools = registry.list_tools();
    for (name, desc) in tools {
        info!("Available tool: {} - {}", name, desc);
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
