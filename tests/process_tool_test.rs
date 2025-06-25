// Tests for process tool to improve coverage
use pcode::tools::process::ProcessTool;
use pcode::tools::{Tool, ToolError};
use serde_json::json;

#[tokio::test]
async fn test_process_with_cwd() {
    let tool = ProcessTool;

    let params = json!({
        "command": "pwd",
        "cwd": "/tmp"
    });

    let result = tool.execute(params).await.unwrap();
    assert_eq!(result["exit_code"], 0);
    assert!(result["stdout"].as_str().unwrap().contains("/tmp"));
}

#[tokio::test]
async fn test_process_with_args() {
    let tool = ProcessTool;

    let params = json!({
        "command": "echo",
        "args": ["hello", "world"]
    });

    let result = tool.execute(params).await.unwrap();
    assert_eq!(result["exit_code"], 0);
    assert_eq!(result["stdout"].as_str().unwrap().trim(), "hello world");
}

#[tokio::test]
async fn test_process_nonzero_exit() {
    let tool = ProcessTool;

    let params = json!({
        "command": "false"  // Always returns exit code 1
    });

    let result = tool.execute(params).await.unwrap();
    assert_eq!(result["exit_code"], 1);
}

#[tokio::test]
async fn test_process_stderr_output() {
    let tool = ProcessTool;

    let params = json!({
        "command": "sh",
        "args": ["-c", "echo 'error' >&2"]
    });

    let result = tool.execute(params).await.unwrap();
    assert_eq!(result["exit_code"], 0);
    assert!(result["stderr"].as_str().unwrap().contains("error"));
}

#[tokio::test]
async fn test_process_nonexistent_command() {
    let tool = ProcessTool;

    let params = json!({
        "command": "nonexistent_command_xyz123"
    });

    let result = tool.execute(params).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ToolError::Execution(_)));
}

#[tokio::test]
async fn test_process_metadata() {
    let tool = ProcessTool;
    assert_eq!(tool.name(), "process");
    assert_eq!(tool.description(), "Execute a system process");
}
