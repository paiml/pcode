use pcode::mcp::{McpProtocol, McpRequest};
use serde_json::json;

#[tokio::test]
async fn test_mcp_protocol_basic() {
    let protocol = McpProtocol::new();

    let request = McpRequest {
        id: "test-1".to_string(),
        method: "unknown".to_string(),
        params: json!({}),
    };

    let response = protocol.process_request(request).await;
    assert_eq!(response.id, "test-1");
    assert!(response.error.is_some());
}
