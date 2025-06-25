pub mod config;
pub mod mcp;
pub mod runtime;
pub mod security;
pub mod token_estimation;
pub mod tools;

pub use mcp::McpProtocol;
pub use runtime::Runtime;
pub use security::SecurityContext;

#[derive(Debug, thiserror::Error)]
pub enum PcodeError {
    #[error("Runtime error: {0}")]
    Runtime(#[from] runtime::RuntimeError),

    #[error("Security error: {0}")]
    Security(#[from] security::SecurityError),

    #[error("MCP protocol error: {0}")]
    Mcp(#[from] mcp::McpError),

    #[error("Tool error: {0}")]
    Tool(#[from] tools::ToolError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, PcodeError>;
