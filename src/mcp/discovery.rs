use super::{ToolManifest, ToolDefinition};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Discovery failed: {0}")]
    Failed(String),
    
    #[error("Timeout during discovery")]
    Timeout,
    
    #[error("No tools found")]
    NoToolsFound,
}

/// Main tool discovery coordinator with fallback strategies
pub struct RobustToolDiscovery {
    strategies: Vec<Box<dyn DiscoveryStrategy>>,
    timeout: Duration,
}

#[async_trait]
pub trait DiscoveryStrategy: Send + Sync {
    /// Attempt to discover tools
    async fn discover(&self) -> Result<Vec<ToolManifest>, DiscoveryError>;
    
    /// Priority of this strategy (higher = try first)
    fn priority(&self) -> u8;
    
    /// Name for logging
    fn name(&self) -> &str;
}

impl Default for RobustToolDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl RobustToolDiscovery {
    pub fn new() -> Self {
        let strategies: Vec<Box<dyn DiscoveryStrategy>> = vec![
            Box::new(McpServerDiscovery::new()),
            Box::new(ConfigFileDiscovery::new()),
            Box::new(EnvironmentDiscovery::new()),
            Box::new(WellKnownPathsDiscovery::new()),
            Box::new(BuiltinToolsDiscovery::new()),
        ];
        
        Self {
            strategies,
            timeout: Duration::from_secs(5),
        }
    }
    
    /// Discover tools using all available strategies
    pub async fn discover_all(&mut self) -> Result<Vec<ToolManifest>, DiscoveryError> {
        // Sort strategies by priority
        self.strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));
        
        let mut all_tools = Vec::new();
        let mut errors = Vec::new();
        
        for strategy in &self.strategies {
            info!("Trying discovery strategy: {}", strategy.name());
            
            match tokio::time::timeout(self.timeout, strategy.discover()).await {
                Ok(Ok(tools)) => {
                    info!("Strategy {} found {} tools", strategy.name(), tools.len());
                    all_tools.extend(tools);
                }
                Ok(Err(e)) => {
                    warn!("Strategy {} failed: {}", strategy.name(), e);
                    errors.push(e);
                }
                Err(_) => {
                    warn!("Strategy {} timed out", strategy.name());
                    errors.push(DiscoveryError::Timeout);
                }
            }
        }
        
        if all_tools.is_empty() && !errors.is_empty() {
            return Err(DiscoveryError::NoToolsFound);
        }
        
        // Deduplicate tools by ID
        let mut seen = std::collections::HashSet::new();
        all_tools.retain(|tool| seen.insert(tool.id.clone()));
        
        Ok(all_tools)
    }
}

/// Discover tools from MCP servers
struct McpServerDiscovery {
    socket_paths: Vec<PathBuf>,
}

impl McpServerDiscovery {
    fn new() -> Self {
        let mut socket_paths = vec![
            PathBuf::from("/tmp/mcp.sock"),
            PathBuf::from("/var/run/mcp/server.sock"),
        ];
        
        // Add user-specific paths
        if let Ok(home) = std::env::var("HOME") {
            socket_paths.push(PathBuf::from(format!("{}/.mcp/server.sock", home)));
        }
        
        Self { socket_paths }
    }
}

#[async_trait]
impl DiscoveryStrategy for McpServerDiscovery {
    async fn discover(&self) -> Result<Vec<ToolManifest>, DiscoveryError> {
        for path in &self.socket_paths {
            if path.exists() {
                debug!("Checking MCP server at {:?}", path);
                // In a real implementation, connect to the socket and query for tools
                // For now, return empty to continue with other strategies
            }
        }
        
        Ok(vec![])
    }
    
    fn priority(&self) -> u8 {
        100 // Highest priority
    }
    
    fn name(&self) -> &str {
        "MCP Server Discovery"
    }
}

/// Discover tools from configuration files
struct ConfigFileDiscovery {
    config_paths: Vec<PathBuf>,
}

impl ConfigFileDiscovery {
    fn new() -> Self {
        let mut config_paths = vec![];
        
        // System-wide configs
        config_paths.push(PathBuf::from("/etc/pcode/tools.json"));
        
        // User configs
        if let Ok(home) = std::env::var("HOME") {
            config_paths.push(PathBuf::from(format!("{}/.config/pcode/tools.json", home)));
            config_paths.push(PathBuf::from(format!("{}/.pcode/tools.json", home)));
        }
        
        // Local project config
        config_paths.push(PathBuf::from(".pcode/tools.json"));
        config_paths.push(PathBuf::from("pcode.tools.json"));
        
        Self { config_paths }
    }
}

#[async_trait]
impl DiscoveryStrategy for ConfigFileDiscovery {
    async fn discover(&self) -> Result<Vec<ToolManifest>, DiscoveryError> {
        for path in &self.config_paths {
            if path.exists() {
                debug!("Reading tool config from {:?}", path);
                
                match tokio::fs::read_to_string(path).await {
                    Ok(content) => {
                        match serde_json::from_str::<Vec<ToolManifest>>(&content) {
                            Ok(tools) => return Ok(tools),
                            Err(e) => warn!("Failed to parse {}: {}", path.display(), e),
                        }
                    }
                    Err(e) => warn!("Failed to read {}: {}", path.display(), e),
                }
            }
        }
        
        Ok(vec![])
    }
    
    fn priority(&self) -> u8 {
        80
    }
    
    fn name(&self) -> &str {
        "Config File Discovery"
    }
}

/// Discover tools from environment variables
struct EnvironmentDiscovery;

impl EnvironmentDiscovery {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DiscoveryStrategy for EnvironmentDiscovery {
    async fn discover(&self) -> Result<Vec<ToolManifest>, DiscoveryError> {
        let mut tools = Vec::new();
        
        // Check for PCODE_TOOLS environment variable
        if let Ok(tools_json) = std::env::var("PCODE_TOOLS") {
            match serde_json::from_str::<Vec<ToolManifest>>(&tools_json) {
                Ok(env_tools) => {
                    info!("Found {} tools from PCODE_TOOLS env var", env_tools.len());
                    tools.extend(env_tools);
                }
                Err(e) => warn!("Failed to parse PCODE_TOOLS: {}", e),
            }
        }
        
        // Check for individual tool paths
        for (key, value) in std::env::vars() {
            if key.starts_with("PCODE_TOOL_") {
                let tool_name = key.strip_prefix("PCODE_TOOL_").unwrap().to_lowercase();
                debug!("Found tool {} at {}", tool_name, value);
                
                // Create a basic manifest for the tool
                tools.push(ToolManifest {
                    id: tool_name.clone(),
                    name: tool_name,
                    version: "unknown".to_string(),
                    description: format!("Tool from environment variable {}", key),
                    executable: Some(value),
                    tools: vec![],
                });
            }
        }
        
        Ok(tools)
    }
    
    fn priority(&self) -> u8 {
        60
    }
    
    fn name(&self) -> &str {
        "Environment Discovery"
    }
}

/// Discover tools from well-known paths
struct WellKnownPathsDiscovery {
    search_paths: Vec<PathBuf>,
}

impl WellKnownPathsDiscovery {
    fn new() -> Self {
        let mut search_paths = vec![
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/opt/pcode/tools"),
        ];
        
        // Add paths from PATH environment variable
        if let Ok(path_var) = std::env::var("PATH") {
            for path in path_var.split(':') {
                search_paths.push(PathBuf::from(path));
            }
        }
        
        Self { search_paths }
    }
}

#[async_trait]
impl DiscoveryStrategy for WellKnownPathsDiscovery {
    async fn discover(&self) -> Result<Vec<ToolManifest>, DiscoveryError> {
        let mut tools = Vec::new();
        
        // Known tool patterns
        let known_tools = vec![
            ("pmat", "PMAT - AI Model Analysis Tool"),
            ("mcp-server", "MCP Server"),
            ("claude-mcp", "Claude MCP Integration"),
        ];
        
        for (tool_name, description) in known_tools {
            for path in &self.search_paths {
                let tool_path = path.join(tool_name);
                if tool_path.exists() {
                    debug!("Found {} at {:?}", tool_name, tool_path);
                    
                    tools.push(ToolManifest {
                        id: tool_name.to_string(),
                        name: tool_name.to_string(),
                        version: "detected".to_string(),
                        description: description.to_string(),
                        executable: Some(tool_path.to_string_lossy().to_string()),
                        tools: vec![],
                    });
                    
                    break; // Found this tool, move to next
                }
            }
        }
        
        Ok(tools)
    }
    
    fn priority(&self) -> u8 {
        40
    }
    
    fn name(&self) -> &str {
        "Well-Known Paths Discovery"
    }
}

/// Always available built-in tools
struct BuiltinToolsDiscovery;

impl BuiltinToolsDiscovery {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DiscoveryStrategy for BuiltinToolsDiscovery {
    async fn discover(&self) -> Result<Vec<ToolManifest>, DiscoveryError> {
        // Return manifests for our built-in tools
        Ok(vec![
            ToolManifest {
                id: "builtin".to_string(),
                name: "Built-in Tools".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: "Core pcode tools".to_string(),
                executable: None,
                tools: vec![
                    ToolDefinition {
                        name: "bash".to_string(),
                        description: "Execute bash commands".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "command": { "type": "string" },
                                "timeout": { "type": "number" }
                            },
                            "required": ["command"]
                        }),
                    },
                    ToolDefinition {
                        name: "file_read".to_string(),
                        description: "Read file contents".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }),
                    },
                    ToolDefinition {
                        name: "file_write".to_string(),
                        description: "Write file contents".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "content": { "type": "string" }
                            },
                            "required": ["path", "content"]
                        }),
                    },
                    ToolDefinition {
                        name: "llm".to_string(),
                        description: "Query language model".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "prompt": { "type": "string" },
                                "max_tokens": { "type": "number" },
                                "temperature": { "type": "number" }
                            },
                            "required": ["prompt"]
                        }),
                    },
                ],
            },
        ])
    }
    
    fn priority(&self) -> u8 {
        20 // Lowest priority - always available fallback
    }
    
    fn name(&self) -> &str {
        "Built-in Tools"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_builtin_discovery() {
        let discovery = BuiltinToolsDiscovery::new();
        let tools = discovery.discover().await.unwrap();
        
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id, "builtin");
        assert!(!tools[0].tools.is_empty());
    }
    
    #[tokio::test]
    async fn test_robust_discovery() {
        let mut discovery = RobustToolDiscovery::new();
        let tools = discovery.discover_all().await.unwrap();
        
        // Should at least have built-in tools
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.id == "builtin"));
    }
    
    #[tokio::test]
    async fn test_discovery_priority() {
        let strategies: Vec<Box<dyn DiscoveryStrategy>> = vec![
            Box::new(BuiltinToolsDiscovery::new()),
            Box::new(McpServerDiscovery::new()),
            Box::new(ConfigFileDiscovery::new()),
        ];
        
        let priorities: Vec<u8> = strategies.iter().map(|s| s.priority()).collect();
        assert!(priorities[0] < priorities[1]); // Built-in has lower priority
        assert!(priorities[1] > priorities[2]); // MCP has highest priority
    }
}