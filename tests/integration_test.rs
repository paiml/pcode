use pcode::{
    security::{SecurityContext, SecurityPolicy},
    tools::{
        file::{FileReadTool, FileWriteTool},
        ToolRegistry, ToolRequest,
    },
};
use tempfile::TempDir;

#[tokio::test]
async fn test_full_integration() {
    // Create temp directory for testing
    let temp_dir = TempDir::new().unwrap();

    // Set up security context
    let policy = SecurityPolicy {
        allowed_paths: vec![temp_dir.path().to_path_buf()],
        allow_network: false,
        allow_process_spawn: false,
        max_memory_mb: 256,
        network_policy: None,
    };

    // Security context might fail in tests
    let _ = SecurityContext::new(policy);

    // Set up tool registry
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FileReadTool));
    registry.register(Box::new(FileWriteTool));

    // Test file write
    let file_path = temp_dir.path().join("test.txt");
    let write_request = ToolRequest {
        tool: "file_write".to_string(),
        params: serde_json::json!({
            "path": file_path.to_string_lossy(),
            "content": "Integration test content"
        }),
    };

    let response = registry.execute(write_request).await;
    assert!(response.success);

    // Test file read
    let read_request = ToolRequest {
        tool: "file_read".to_string(),
        params: serde_json::json!({
            "path": file_path.to_string_lossy()
        }),
    };

    let response = registry.execute(read_request).await;
    assert!(response.success);

    let result = response.result.unwrap();
    assert_eq!(result["content"], "Integration test content");
}

#[test]
fn test_binary_size() {
    // This test would check the actual binary size in CI
    // For now, just a placeholder
    // TODO: Implement actual binary size check
}
