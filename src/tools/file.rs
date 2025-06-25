use super::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
struct FileReadParams {
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileWriteParams {
    path: String,
    content: String,
    append: Option<bool>,
}

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read contents of a file"
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let params: FileReadParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let path = PathBuf::from(&params.path);
        debug!("Reading file: {:?}", path);

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to read file: {}", e)))?;

        let result = if let (Some(offset), Some(limit)) = (params.offset, params.limit) {
            content
                .lines()
                .skip(offset)
                .take(limit)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            content
        };

        Ok(serde_json::json!({
            "content": result,
            "lines": result.lines().count(),
            "size": result.len()
        }))
    }
}

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file"
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let params: FileWriteParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let path = PathBuf::from(&params.path);
        debug!("Writing to file: {:?}", path);

        if params.append.unwrap_or(false) {
            fs::write(&path, &params.content)
                .await
                .map_err(|e| ToolError::Execution(format!("Failed to write file: {}", e)))?;
        } else {
            fs::write(&path, &params.content)
                .await
                .map_err(|e| ToolError::Execution(format!("Failed to write file: {}", e)))?;
        }

        Ok(serde_json::json!({
            "path": path.to_string_lossy(),
            "size": params.content.len(),
            "success": true
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Test write
        let write_tool = FileWriteTool;
        let write_params = serde_json::json!({
            "path": file_path.to_string_lossy(),
            "content": "Hello, world!"
        });

        let result = write_tool.execute(write_params).await.unwrap();
        assert_eq!(result["success"], true);

        // Test read
        let read_tool = FileReadTool;
        let read_params = serde_json::json!({
            "path": file_path.to_string_lossy()
        });

        let result = read_tool.execute(read_params).await.unwrap();
        assert_eq!(result["content"], "Hello, world!");
    }
}
