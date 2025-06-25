// Tests for error handling to improve coverage
use pcode::{
    PcodeError,
    runtime::RuntimeError,
    security::SecurityError,
    mcp::McpError,
    tools::ToolError,
};

#[test]
fn test_runtime_error_variants() {
    let err = RuntimeError::Creation("init failed".to_string());
    assert!(err.to_string().contains("Failed to create runtime"));
    
    let err = RuntimeError::Execution("exec failed".to_string());
    assert!(err.to_string().contains("Task execution error"));
}

#[test]
fn test_tool_error_variants() {
    let err = ToolError::NotFound("missing".to_string());
    assert!(err.to_string().contains("Tool not found"));
    
    let err = ToolError::InvalidParams("bad params".to_string());
    assert!(err.to_string().contains("Invalid parameters"));
    
    let err = ToolError::PermissionDenied("no access".to_string());
    assert!(err.to_string().contains("Permission denied"));
    
    let err = ToolError::Execution("failed".to_string());
    assert!(err.to_string().contains("Tool execution error"));
}

#[test]
fn test_mcp_error_display() {
    let err = McpError::Protocol("bad protocol".to_string());
    assert_eq!(err.to_string(), "Protocol error: bad protocol");
    
    let err = McpError::Transport("network down".to_string());
    assert_eq!(err.to_string(), "Transport error: network down");
    
    let err = McpError::Serialization("bad json".to_string());
    assert_eq!(err.to_string(), "Serialization error: bad json");
}

#[test]
fn test_pcode_error_chain() {
    // Test that errors properly chain through From implementations
    let tool_err = ToolError::NotFound("test".to_string());
    let pcode_err: PcodeError = tool_err.into();
    assert!(pcode_err.to_string().contains("Tool error"));
    
    let runtime_err = RuntimeError::Creation("test".to_string());
    let pcode_err: PcodeError = runtime_err.into();
    assert!(pcode_err.to_string().contains("Runtime error"));
    
    let sec_err = SecurityError::UnsupportedPlatform;
    let pcode_err: PcodeError = sec_err.into();
    assert!(pcode_err.to_string().contains("Security error"));
    
    let mcp_err = McpError::Protocol("test".to_string());
    let pcode_err: PcodeError = mcp_err.into();
    assert!(pcode_err.to_string().contains("MCP protocol error"));
}

#[test]
fn test_debug_implementations() {
    // Ensure Debug trait is implemented for all error types
    let err = PcodeError::Runtime(RuntimeError::Creation("test".to_string()));
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("Runtime"));
    
    let err = SecurityError::InitError("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("InitError"));
    
    let err = McpError::Transport("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("Transport"));
    
    let err = ToolError::Execution("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("Execution"));
}

#[test]
fn test_pcode_error_other_and_io() {
    // Test Other variant
    let err = PcodeError::Other("custom error".to_string());
    assert!(err.to_string().contains("Other error: custom error"));
    
    // Test IO error conversion
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let pcode_err: PcodeError = io_err.into();
    assert!(pcode_err.to_string().contains("IO error"));
}