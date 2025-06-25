use pcode::tools::ToolRegistry;
use serde_json::json;

#[tokio::test]
async fn test_pmat_complexity_analysis() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(pcode::tools::pmat::PmatTool::new()));

    // Test complexity on a Rust file
    let request = pcode::tools::ToolRequest {
        tool: "pmat".to_string(),
        params: json!({
            "command": "complexity",
            "path": "src/chat.rs"
        }),
    };

    let response = registry.execute(request).await;
    assert!(response.success, "Failed: {:?}", response.error);

    let result = response.result.unwrap();
    assert!(result["summary"]["total_functions"].as_u64().unwrap() > 0);
    assert!(result["files"].is_array());
}

#[tokio::test]
async fn test_pmat_satd_detection() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(pcode::tools::pmat::PmatTool::new()));

    // Test SATD on source directory
    let request = pcode::tools::ToolRequest {
        tool: "pmat".to_string(),
        params: json!({
            "command": "satd",
            "path": "src/"
        }),
    };

    let response = registry.execute(request).await;
    assert!(response.success, "Failed: {:?}", response.error);

    let result = response.result.unwrap();
    assert!(result["summary"]["total_items"].is_u64());
}

#[tokio::test]
async fn test_pmat_dead_code_analysis() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(pcode::tools::pmat::PmatTool::new()));

    // Test coverage on tests directory
    let request = pcode::tools::ToolRequest {
        tool: "pmat".to_string(),
        params: json!({
            "command": "dead-code",
            "path": "tests/"
        }),
    };

    let response = registry.execute(request).await;
    assert!(response.success, "Failed: {:?}", response.error);

    let result = response.result.unwrap();

    // Check that dead code analysis returns expected structure
    assert!(result.is_object());
}

#[tokio::test]
async fn test_pmat_tdg_analysis() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(pcode::tools::pmat::PmatTool::new()));

    // Test TDG on tests directory
    let request = pcode::tools::ToolRequest {
        tool: "pmat".to_string(),
        params: json!({
            "command": "tdg",
            "path": "tests/"
        }),
    };

    let response = registry.execute(request).await;
    assert!(response.success, "Failed: {:?}", response.error);

    let result = response.result.unwrap();

    // Check that TDG analysis returns expected structure
    assert!(result["summary"].is_object() || result["tdg_scores"].is_array());
}

#[tokio::test]
async fn test_pmat_invalid_command() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(pcode::tools::pmat::PmatTool::new()));

    let request = pcode::tools::ToolRequest {
        tool: "pmat".to_string(),
        params: json!({
            "command": "invalid_command",
            "path": "src/"
        }),
    };

    let response = registry.execute(request).await;
    assert!(!response.success);
    assert!(response.error.unwrap().contains("error"));
}

#[tokio::test]
async fn test_pmat_complexity_violations() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(pcode::tools::pmat::PmatTool::new()));

    // Test on a file with known complexity violations
    let request = pcode::tools::ToolRequest {
        tool: "pmat".to_string(),
        params: json!({
            "command": "complexity",
            "path": "src/main.rs"
        }),
    };

    let response = registry.execute(request).await;
    assert!(response.success, "Failed: {:?}", response.error);

    let result = response.result.unwrap();
    // Check that violations are properly reported
    if let Some(violations) = result["violations"].as_array() {
        for violation in violations {
            // Check for either cyclomatic or cognitive complexity
            assert!(violation["value"].is_number() || violation["complexity"].is_number());
        }
    }
}

#[tokio::test]
async fn test_pmat_path_validation() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(pcode::tools::pmat::PmatTool::new()));

    // Test with absolute path outside workspace
    let request = pcode::tools::ToolRequest {
        tool: "pmat".to_string(),
        params: json!({
            "command": "complexity",
            "path": "/etc/passwd"
        }),
    };

    let response = registry.execute(request).await;
    assert!(!response.success);
    assert!(response.error.unwrap().contains("must be within workspace"));
}
