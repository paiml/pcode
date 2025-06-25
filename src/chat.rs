use crate::{
    config::Config,
    context::{PROJECT_CONTEXT, SYSTEM_PROMPT},
    tools::{ToolRegistry, ToolRequest, ToolResponse},
};
use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde_json::json;
use tracing::error;

pub struct InteractiveChat {
    registry: ToolRegistry,
    config: Config,
    history_file: String,
}

impl InteractiveChat {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            config: Config::from_env(),
            history_file: ".pcode_history".to_string(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // Initialize readline editor
        let mut rl = DefaultEditor::new()?;

        // Load history if exists
        let _ = rl.load_history(&self.history_file);

        println!(
            "🤖 pcode v{} - AI Code Assistant",
            env!("CARGO_PKG_VERSION")
        );
        println!("Type 'help' for available commands, 'exit' to quit");
        println!();

        loop {
            let readline = rl.readline("pcode> ");
            match readline {
                Ok(line) => {
                    let line = line.trim();

                    // Add to history
                    let _ = rl.add_history_entry(line);

                    // Handle special commands
                    match line {
                        "" => continue,
                        "exit" | "quit" => {
                            println!("👋 Goodbye!");
                            break;
                        }
                        "help" | "?" => {
                            self.show_help();
                            continue;
                        }
                        "tools" => {
                            self.list_tools();
                            continue;
                        }
                        "clear" => {
                            print!("\x1B[2J\x1B[1;1H"); // Clear screen
                            continue;
                        }
                        _ => {}
                    }

                    // Process user input
                    if let Err(e) = self.process_input(line).await {
                        error!("Error processing input: {}", e);
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("^D");
                    break;
                }
                Err(err) => {
                    error!("Error reading input: {:?}", err);
                    break;
                }
            }
        }

        // Save history
        let _ = rl.save_history(&self.history_file);

        Ok(())
    }

    async fn build_enhanced_prompt(&self, input: &str) -> String {
        if input.to_lowercase().contains("readme") {
            // Read README.md and include it in context
            let readme_content = match self.read_file("README.md").await {
                Ok(content) => format!("\n\nREADME.md content:\n{}", content),
                Err(_) => String::new(),
            };
            format!(
                "{}\n\nContext:\n{}{}\n\nUser: {}\n\nAssistant:",
                SYSTEM_PROMPT, PROJECT_CONTEXT, readme_content, input
            )
        } else {
            format!(
                "{}\n\nContext:\n{}\n\nUser: {}\n\nAssistant:",
                SYSTEM_PROMPT, PROJECT_CONTEXT, input
            )
        }
    }

    fn print_llm_response(&self, response: ToolResponse) -> Result<()> {
        if response.success {
            if let Some(result) = response.result {
                if let Some(text) = result.get("response").and_then(|v| v.as_str()) {
                    println!("{}", text);
                } else {
                    println!("🤖 {}", serde_json::to_string_pretty(&result)?);
                }
            } else {
                println!("💭 No response from LLM");
            }
        } else {
            println!(
                "❌ Error: {}",
                response
                    .error
                    .unwrap_or_else(|| "Failed to process with LLM".to_string())
            );
        }
        Ok(())
    }

    async fn process_with_llm(&self, input: &str) -> Result<()> {
        let enhanced_prompt = self.build_enhanced_prompt(input).await;

        let request = ToolRequest {
            tool: "llm".to_string(),
            params: json!({
                "prompt": enhanced_prompt,
                "max_tokens": 800,
                "temperature": 0.7
            }),
        };

        let response = self.registry.execute(request).await;
        self.print_llm_response(response)
    }

    async fn process_input(&self, input: &str) -> Result<()> {
        // Check if this is a direct tool command
        if input.starts_with('/') {
            return self.execute_tool_command(input).await;
        }

        // Process natural language input with LLM if available
        if self.config.has_api_key() {
            self.process_with_llm(input).await
        } else {
            // Provide helpful responses without LLM
            self.handle_offline_query(input)
        }
    }

    fn show_about_info(&self) {
        println!("🤖 pcode is a production-grade AI code agent with extreme performance and security requirements.\n");
        println!("Key features:");
        println!("• Interactive chat interface for AI-assisted coding");
        println!(
            "• Security sandboxing (Landlock on Linux, platform-specific on macOS/Windows)"
        );
        println!("• Tool system for file operations, process execution, and more");
        println!("• Token estimation with perfect hash tables");
        println!("• Extreme performance: <200ms latency, <12MB binary size");
        println!("\nSet AI_STUDIO_API_KEY environment variable to enable AI features.");
    }

    fn show_no_api_key_message(&self) {
        println!(
            "ℹ️  No AI Studio API key found. Set AI_STUDIO_API_KEY to enable AI responses."
        );
        println!("   Type 'help' for available commands or 'tools' to see available tools.");
    }

    fn is_about_query(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();
        input_lower.contains("about")
            && (input_lower.contains("project") || input_lower.contains("pcode"))
    }

    fn handle_offline_query(&self, input: &str) -> Result<()> {
        let input_lower = input.to_lowercase();

        if self.is_about_query(input) {
            self.show_about_info();
        } else if input_lower.contains("help") {
            self.show_help();
        } else if input_lower.contains("tool") {
            self.list_tools();
        } else {
            self.show_no_api_key_message();
        }

        Ok(())
    }

    fn parse_file_write_params(&self, params_str: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = params_str.splitn(2, ' ').collect();
        if parts.len() == 2 {
            Some(json!({ "path": parts[0], "content": parts[1] }))
        } else {
            println!("❌ Usage: /file_write <path> <content>");
            None
        }
    }

    fn parse_process_params(&self, params_str: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = params_str.split_whitespace().collect();
        if parts.is_empty() {
            println!("❌ Usage: /process <command> [args...]");
            None
        } else {
            let command = parts[0];
            let args = if parts.len() > 1 {
                Some(parts[1..].to_vec())
            } else {
                None
            };
            Some(json!({ "command": command, "args": args }))
        }
    }

    fn parse_pmat_params(&self, params_str: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = params_str.split_whitespace().collect();
        if parts.len() < 2 {
            println!("❌ Usage: /pmat <command> <path>");
            println!("   Commands: complexity, satd, coverage, tdg");
            None
        } else {
            Some(json!({ "command": parts[0], "path": parts[1] }))
        }
    }

    fn parse_dev_cli_params(&self, params_str: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = params_str.split_whitespace().collect();
        if parts.is_empty() {
            println!("❌ Usage: /dev_cli <tool> [args...]");
            println!("   Tools: rg, fd, cargo, git, make, etc.");
            None
        } else {
            Some(json!({
                "tool": parts[0],
                "args": parts[1..].to_vec()
            }))
        }
    }

    fn parse_fix_params(&self, params_str: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = params_str.split_whitespace().collect();
        if parts.len() < 2 {
            println!("❌ Usage: /fix <type> <path> [--dry-run]");
            println!("   Types: complexity, format, lint");
            None
        } else {
            let dry_run = parts.get(2).is_some_and(|&s| s == "--dry-run");
            Some(json!({
                "fix_type": parts[0],
                "path": parts[1],
                "dry_run": dry_run
            }))
        }
    }

    fn parse_tool_params(
        &self,
        tool_name: &str,
        params_str: &str,
    ) -> Result<Option<serde_json::Value>> {
        match tool_name {
            "file_read" => Ok(Some(json!({ "path": params_str }))),
            "file_write" => Ok(self.parse_file_write_params(params_str)),
            "process" => Ok(self.parse_process_params(params_str)),
            "llm" => Ok(Some(json!({ "prompt": params_str }))),
            "token_estimate" => Ok(Some(json!({ "text": params_str }))),
            "pmat" => Ok(self.parse_pmat_params(params_str)),
            "bash" => Ok(Some(json!({ "command": params_str }))),
            "dev_cli" => Ok(self.parse_dev_cli_params(params_str)),
            "fix" => Ok(self.parse_fix_params(params_str)),
            _ => {
                println!("❌ Unknown parameter format for tool: {}", tool_name);
                Ok(None)
            }
        }
    }

    async fn execute_tool(&self, tool_name: &str, params: serde_json::Value) -> Result<()> {
        let request = ToolRequest {
            tool: tool_name.to_string(),
            params,
        };

        println!("🔧 Executing tool: {}", tool_name);
        let response = self.registry.execute(request).await;

        if response.success {
            if let Some(result) = response.result {
                println!("✅ Success:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("✅ Success (no output)");
            }
        } else {
            println!(
                "❌ Error: {}",
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        Ok(())
    }

    async fn execute_tool_command(&self, input: &str) -> Result<()> {
        let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
        if parts.is_empty() {
            println!("❌ Invalid command");
            return Ok(());
        }

        let tool_name = parts[0];
        let params_str = parts.get(1).unwrap_or(&"{}");

        // Parse parameters
        let params = if params_str.starts_with('{') {
            // JSON parameters
            Some(serde_json::from_str(params_str)?)
        } else {
            // Simple parameter handling
            self.parse_tool_params(tool_name, params_str)?
        };

        if let Some(params) = params {
            self.execute_tool(tool_name, params).await
        } else {
            Ok(())
        }
    }

    fn show_help(&self) {
        println!("\n📚 Available Commands:");
        println!("  help, ?        - Show this help message");
        println!("  tools          - List available tools");
        println!("  clear          - Clear the screen");
        println!("  exit, quit     - Exit pcode");
        println!();
        println!("🔧 Tool Commands:");
        println!("  /file_read <path>               - Read a file");
        println!("  /file_write <path> <content>    - Write to a file");
        println!("  /process <command>              - Execute a command");
        println!("  /llm <prompt>                   - Query the LLM (requires API key)");
        println!("  /token_estimate <text>          - Estimate token count");
        println!("  /pmat <command> <path>          - Run PMAT analysis (complexity, satd, coverage, tdg)");
        println!("  /bash <command>                 - Execute bash commands");
        println!("  /dev_cli <tool> [args...]       - Run dev tools (rg, cargo, git, etc.)");
        println!("  /fix <type> <path> [--dry-run] - Fix code issues (complexity, format, lint)");
        println!();
        println!("💡 Tips:");
        println!("  - Use Tab for command completion");
        println!("  - Use ↑/↓ for command history");
        println!("  - Set AI_STUDIO_API_KEY for LLM features");
        println!();
    }

    fn list_tools(&self) {
        println!("\n🔧 Available Tools:");
        for (name, desc) in self.registry.list_tools() {
            println!("  {} - {}", name, desc);
        }
        println!();
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        let request = ToolRequest {
            tool: "file_read".to_string(),
            params: json!({ "path": path }),
        };

        let response = self.registry.execute(request).await;

        if response.success {
            if let Some(result) = response.result {
                if let Some(content) = result.get("content").and_then(|v| v.as_str()) {
                    return Ok(content.to_string());
                }
            }
        }

        Err(anyhow::anyhow!("Failed to read file"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_chat_creation() {
        let registry = ToolRegistry::new();
        let chat = InteractiveChat::new(registry);
        assert_eq!(chat.history_file, ".pcode_history");
    }
}
