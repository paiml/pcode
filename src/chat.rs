use crate::{
    config::Config,
    tools::{ToolRegistry, ToolRequest},
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

        println!("🤖 pcode v{} - AI Code Assistant", env!("CARGO_PKG_VERSION"));
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

    async fn process_input(&self, input: &str) -> Result<()> {
        // Check if this is a direct tool command
        if input.starts_with('/') {
            return self.execute_tool_command(input).await;
        }

        // For now, echo the input as we don't have LLM integration yet
        println!("🤔 Received: {}", input);
        
        // If we have an API key, we could use the LLM tool
        if self.config.has_api_key() {
            println!("💡 I would process this with the LLM, but full chat integration is not yet implemented.");
            println!("   You can use /llm <prompt> to test the LLM tool directly.");
        } else {
            println!("ℹ️  No AI Studio API key found. Set AI_STUDIO_API_KEY to enable LLM features.");
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
            serde_json::from_str(params_str)?
        } else {
            // Simple parameter handling for common tools
            match tool_name {
                "file_read" => json!({ "path": params_str }),
                "file_write" => {
                    let parts: Vec<&str> = params_str.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        json!({ "path": parts[0], "content": parts[1] })
                    } else {
                        println!("❌ Usage: /file_write <path> <content>");
                        return Ok(());
                    }
                }
                "process" => json!({ "command": params_str }),
                "llm" => json!({ "prompt": params_str }),
                "token_estimate" => json!({ "text": params_str }),
                _ => {
                    println!("❌ Unknown parameter format for tool: {}", tool_name);
                    return Ok(());
                }
            }
        };

        // Execute tool
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
            println!("❌ Error: {}", response.error.unwrap_or_else(|| "Unknown error".to_string()));
        }

        Ok(())
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