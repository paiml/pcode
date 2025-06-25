use pcode::tools::{Tool, ToolError, ToolRegistry, ToolRequest};
use async_trait::async_trait;
use serde_json::json;

struct FailingTool;

#[async_trait]
impl Tool for FailingTool {
    fn name(&self) -> &str {
        "failing_tool"
    }
    
    fn description(&self) -> &str {
        "A tool that always fails"
    }
    
    async fn execute(&self, _params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        Err(ToolError::Execution("Intentional failure".to_string()))
    }
}

#[tokio::test]
async fn test_tool_registry_error_handling() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FailingTool));
    
    let request = ToolRequest {
        tool: "failing_tool".to_string(),
        params: json!({}),
    };
    
    let response = registry.execute(request).await;
    assert!(!response.success);
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("Intentional failure"));
}

#[tokio::test]
async fn test_tool_registry_list() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FailingTool));
    
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].0, "failing_tool");
    assert_eq!(tools[0].1, "A tool that always fails");
}

#[test]
fn test_tool_registry_default() {
    let registry = ToolRegistry::default();
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 0);
}

#[test]
fn test_tool_error_variants() {
    let err1 = ToolError::Execution("test".to_string());
    assert_eq!(err1.to_string(), "Tool execution error: test");
    
    let err2 = ToolError::InvalidParams("bad params".to_string());
    assert_eq!(err2.to_string(), "Invalid parameters: bad params");
    
    let err3 = ToolError::PermissionDenied("no access".to_string());
    assert_eq!(err3.to_string(), "Permission denied: no access");
    
    let err4 = ToolError::NotFound("missing".to_string());
    assert_eq!(err4.to_string(), "Tool not found: missing");
}