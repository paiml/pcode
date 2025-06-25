use anyhow::Result;
use clap::Parser;
use pcode::{
    chat::InteractiveChat,
    config::Config,
    mcp::{discovery::RobustToolDiscovery, McpProtocol},
    runtime::Runtime,
    security::{SecurityContext, SecurityPolicy},
    tools::{
        bash::BashTool,
        coverage::CoverageTool,
        dev_cli::DevCliTool,
        file::{FileReadTool, FileWriteTool},
        fix::FixTool,
        llm::{LlmTool, TokenEstimateTool},
        pmat::PmatTool,
        process::ProcessTool,
        refactor::RefactorTool,
        ToolRegistry,
    },
};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "pcode")]
#[command(about = "Production-grade AI code agent", long_about = None)]
struct Args {
    #[arg(short, long, help = "Working directory")]
    workdir: Option<PathBuf>,

    #[arg(short, long, help = "Enable debug logging")]
    debug: bool,

    #[arg(short = 'V', long, help = "Print version information")]
    version: bool,

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

    // Handle version flag
    if args.version {
        println!("pcode {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

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

fn parse_tool_params(tool_name: &str, params_str: &str) -> Result<serde_json::Value> {
    use serde_json::json;

    match tool_name {
        "file_read" => Ok(json!({ "path": params_str })),
        "file_write" => {
            let parts: Vec<&str> = params_str.splitn(2, ' ').collect();
            if parts.len() == 2 {
                Ok(json!({ "path": parts[0], "content": parts[1] }))
            } else {
                anyhow::bail!("Usage: /file_write <path> <content>");
            }
        }
        "process" => {
            let parts: Vec<&str> = params_str.split_whitespace().collect();
            if parts.is_empty() {
                anyhow::bail!("Usage: /process <command> [args...]");
            }
            Ok(json!({
                "command": parts[0],
                "args": if parts.len() > 1 { Some(parts[1..].to_vec()) } else { None }
            }))
        }
        "llm" => Ok(json!({ "prompt": params_str })),
        "token_estimate" => Ok(json!({ "text": params_str })),
        "pmat" => {
            let parts: Vec<&str> = params_str.split_whitespace().collect();
            if parts.len() < 2 {
                anyhow::bail!("Usage: /pmat <command> <path>");
            }
            Ok(json!({ "command": parts[0], "path": parts[1] }))
        }
        "bash" => Ok(json!({ "command": params_str })),
        "dev_cli" => {
            let parts: Vec<&str> = params_str.split_whitespace().collect();
            if parts.is_empty() {
                anyhow::bail!("Usage: /dev_cli <tool> [args...]");
            }
            Ok(json!({ "tool": parts[0], "args": parts[1..].to_vec() }))
        }
        "fix" => {
            let parts: Vec<&str> = params_str.split_whitespace().collect();
            if parts.len() < 2 {
                anyhow::bail!("Usage: /fix <type> <path> [--dry-run]");
            }
            let dry_run = parts.get(2).is_some_and(|&s| s == "--dry-run");
            Ok(json!({
                "fix_type": parts[0],
                "path": parts[1],
                "dry_run": dry_run
            }))
        }
        _ => anyhow::bail!("Unknown tool: {}", tool_name),
    }
}

fn format_tool_result(tool_name: &str, result: &serde_json::Value) -> Result<String> {
    if tool_name == "llm" {
        if let Some(text) = result.get("response").and_then(|v| v.as_str()) {
            return Ok(text.to_string());
        }
    }
    serde_json::to_string_pretty(result).map_err(Into::into)
}

async fn execute_tool_command(
    registry: ToolRegistry,
    tool_name: &str,
    params: serde_json::Value,
) -> Result<()> {
    use pcode::tools::ToolRequest;

    let request = ToolRequest {
        tool: tool_name.to_string(),
        params,
    };

    let response = registry.execute(request).await;

    if !response.success {
        let error_msg = response
            .error
            .unwrap_or_else(|| "Unknown error".to_string());
        anyhow::bail!("Error: {}", error_msg);
    }

    if let Some(result) = response.result {
        let formatted = format_tool_result(tool_name, &result)?;
        println!("{}", formatted);
    }

    Ok(())
}

async fn execute_single_command(registry: ToolRegistry, command: &str) -> Result<()> {
    use serde_json::json;

    if let Some(stripped) = command.strip_prefix('/') {
        // Parse tool command
        let parts: Vec<&str> = stripped.splitn(2, ' ').collect();
        if parts.is_empty() {
            anyhow::bail!("Invalid command");
        }

        let tool_name = parts[0];
        let params_str = parts.get(1).unwrap_or(&"{}");
        let params = parse_tool_params(tool_name, params_str)?;

        execute_tool_command(registry, tool_name, params).await
    } else {
        // Natural language command - use LLM if available
        let config = Config::from_env();
        if config.has_api_key() {
            let params = json!({
                "prompt": command,
                "max_tokens": 800
            });
            execute_tool_command(registry, "llm", params).await
        } else {
            println!("No AI Studio API key found. Use tool commands starting with '/' or set AI_STUDIO_API_KEY.");
            Ok(())
        }
    }
}

async fn async_main(args: Args) -> Result<()> {
    // Set up security context
    if !args.no_sandbox {
        let policy = SecurityPolicy {
            allowed_paths: vec![args.workdir.clone().unwrap_or_else(|| PathBuf::from("."))],
            allow_network: false,
            allow_process_spawn: true,
            max_memory_mb: args.max_memory,
            network_policy: None,
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

    // Initialize tool registry with discovery
    let registry = initialize_tool_registry().await?;
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

async fn initialize_tool_registry() -> Result<ToolRegistry> {
    let mut registry = ToolRegistry::new();

    // Register built-in tools first
    registry.register(Box::new(FileReadTool));
    registry.register(Box::new(FileWriteTool));
    registry.register(Box::new(ProcessTool));
    registry.register(Box::new(LlmTool::new()));
    registry.register(Box::new(TokenEstimateTool));
    registry.register(Box::new(PmatTool::new()));
    registry.register(Box::new(BashTool::new()));
    registry.register(Box::new(DevCliTool::new()));
    registry.register(Box::new(FixTool::new()));
    registry.register(Box::new(CoverageTool::new()));
    registry.register(Box::new(RefactorTool::new()));

    debug!("Registered {} built-in tools", registry.list_tools().len());

    // Discover additional tools
    let mut discovery = RobustToolDiscovery::new();
    match discovery.discover_all().await {
        Ok(manifests) => {
            info!("Discovered {} tool manifests", manifests.len());
            // In a real implementation, we would create tool wrappers for discovered tools
            // For now, we just log them
            for manifest in manifests {
                debug!(
                    "Discovered tool manifest: {} v{}",
                    manifest.name, manifest.version
                );
                for tool in &manifest.tools {
                    debug!("  - Tool: {}", tool.name);
                }
            }
        }
        Err(e) => {
            warn!("Tool discovery failed: {}, using built-in tools only", e);
        }
    }

    Ok(registry)
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
