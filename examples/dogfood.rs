use pcode::tools::{file::FileReadTool, process::ProcessTool, Tool, ToolRegistry};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🐕 Dogfooding pcode to improve its own coverage!");

    // Create tools
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FileReadTool));
    registry.register(Box::new(ProcessTool));

    // Use file_read tool to analyze our own source
    println!("\n📖 Reading uncovered code in mcp/transport.rs...");
    let file_tool = FileReadTool;
    let params = json!({
        "path": "src/mcp/transport.rs",
        "offset": 30,
        "limit": 40
    });

    match file_tool.execute(params).await {
        Ok(result) => {
            println!("Found {} lines of code", result["lines"]);
            println!(
                "Content preview:\n{}",
                result["content"].as_str().unwrap_or("")
            );
        }
        Err(e) => println!("Error reading file: {}", e),
    }

    // Use process tool to run coverage analysis
    println!("\n📊 Running coverage analysis...");
    let process_tool = ProcessTool;
    let params = json!({
        "command": "cargo",
        "args": ["tarpaulin", "--lib", "--print-summary"],
        "timeout_ms": 30000
    });

    match process_tool.execute(params).await {
        Ok(result) => {
            if result["success"].as_bool().unwrap_or(false) {
                let output = result["stdout"].as_str().unwrap_or("");
                // Extract coverage percentage
                if let Some(line) = output.lines().find(|l| l.contains("% coverage")) {
                    println!("Current coverage: {}", line.trim());
                }
            } else {
                println!("Coverage command failed: {}", result["stderr"]);
            }
        }
        Err(e) => println!("Error running coverage: {}", e),
    }

    // Generate test suggestions
    println!("\n💡 Test suggestions for improving coverage:");
    println!("1. Add async tests for StdioTransport::send and receive");
    println!("2. Add error case tests for protocol message decoding");
    println!("3. Add platform-specific security tests with #[cfg(...)]");
    println!("4. Mock external dependencies for better async testing");

    Ok(())
}
