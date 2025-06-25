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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        // Test From implementations
        let runtime_err = runtime::RuntimeError::Creation("test".to_string());
        let pcode_err: PcodeError = runtime_err.into();
        assert!(matches!(pcode_err, PcodeError::Runtime(_)));

        let security_err = security::SecurityError::InitError("test".to_string());
        let pcode_err: PcodeError = security_err.into();
        assert!(matches!(pcode_err, PcodeError::Security(_)));

        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let pcode_err: PcodeError = io_err.into();
        assert!(matches!(pcode_err, PcodeError::Io(_)));
    }

    #[test]
    fn test_error_display() {
        let err = PcodeError::Other("Custom error".to_string());
        assert_eq!(err.to_string(), "Other error: Custom error");
    }
}
