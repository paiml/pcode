use super::{Tool, ToolError};
use crate::mcp::streaming::{StreamRequest, StreamResponse, StreamingMode, StreamingTool};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Parameters for streaming command execution
#[derive(Debug, Serialize, Deserialize)]
struct StreamExecParams {
    command: String,
    args: Vec<String>,
    stream_output: bool,
    stream_input: bool,
}

/// Streaming command execution tool
pub struct StreamExecTool;

impl StreamExecTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StreamExecTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for StreamExecTool {
    fn name(&self) -> &str {
        "stream_exec"
    }

    fn description(&self) -> &str {
        "Execute commands with streaming I/O support"
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let params: StreamExecParams =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        if !params.stream_output && !params.stream_input {
            // Non-streaming execution
            let output = Command::new(&params.command)
                .args(&params.args)
                .output()
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            return Ok(serde_json::json!({
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
                "exit_code": output.status.code(),
                "streaming": false
            }));
        }

        // For streaming, return a handle to the stream
        Ok(serde_json::json!({
            "streaming": true,
            "mode": if params.stream_input && params.stream_output {
                "both"
            } else if params.stream_input {
                "input"
            } else {
                "output"
            },
            "message": "Use streaming protocol to interact with this command"
        }))
    }
}

#[async_trait]
impl StreamingTool for StreamExecTool {
    fn streaming_mode(&self) -> StreamingMode {
        StreamingMode::Both
    }

    async fn process_stream(
        &self,
        mut input: Pin<Box<dyn Stream<Item = StreamRequest> + Send>>,
        output: mpsc::Sender<StreamResponse>,
    ) -> Result<(), crate::mcp::McpError> {
        // Get the first message which should contain the command
        let first = input
            .next()
            .await
            .ok_or_else(|| crate::mcp::McpError::Protocol("No initial message".to_string()))?;

        // Parse command from first message
        let params: StreamExecParams = serde_json::from_slice(&first.data)
            .map_err(|e| crate::mcp::McpError::Protocol(format!("Invalid params: {}", e)))?;

        info!(
            "Starting streaming execution of: {} {:?}",
            params.command, params.args
        );

        // Start the process
        let mut child = Command::new(&params.command)
            .args(&params.args)
            .stdin(if params.stream_input {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(if params.stream_output {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| crate::mcp::McpError::Transport(format!("Failed to spawn: {}", e)))?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Handle output streaming
        if params.stream_output {
            if let Some(stdout) = stdout {
                let output_tx = output.clone();
                let stream_id = first.id;

                tokio::spawn(async move {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    let mut sequence = 0u32;

                    while let Ok(Some(line)) = lines.next_line().await {
                        let response = StreamResponse {
                            id: stream_id,
                            sequence,
                            data: format!("{}\n", line).into_bytes(),
                            is_last: false,
                            error: None,
                        };

                        if output_tx.send(response).await.is_err() {
                            break;
                        }
                        sequence += 1;
                    }

                    // Send final message
                    let _ = output_tx
                        .send(StreamResponse {
                            id: stream_id,
                            sequence,
                            data: vec![],
                            is_last: true,
                            error: None,
                        })
                        .await;
                });
            }
        }

        // Handle stderr streaming
        if let Some(stderr) = stderr {
            let output_tx = output.clone();
            let stream_id = first.id;

            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                let mut sequence = 1000u32; // Different sequence range for stderr

                while let Ok(Some(line)) = lines.next_line().await {
                    let response = StreamResponse {
                        id: stream_id,
                        sequence,
                        data: format!("[stderr] {}\n", line).into_bytes(),
                        is_last: false,
                        error: None,
                    };

                    if output_tx.send(response).await.is_err() {
                        break;
                    }
                    sequence += 1;
                }
            });
        }

        // Handle input streaming
        if params.stream_input {
            if let Some(mut stdin) = stdin {
                tokio::spawn(async move {
                    while let Some(request) = input.next().await {
                        if request.is_last {
                            break;
                        }

                        if stdin.write_all(&request.data).await.is_err() {
                            error!("Failed to write to stdin");
                            break;
                        }

                        if stdin.flush().await.is_err() {
                            error!("Failed to flush stdin");
                            break;
                        }
                    }

                    // Close stdin
                    drop(stdin);
                });
            }
        }

        // Wait for process to complete
        let status = child
            .wait()
            .await
            .map_err(|e| crate::mcp::McpError::Transport(format!("Process wait failed: {}", e)))?;

        debug!("Process exited with status: {:?}", status);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn test_non_streaming_exec() {
        let tool = StreamExecTool::new();
        let params = serde_json::json!({
            "command": "echo",
            "args": ["hello world"],
            "stream_output": false,
            "stream_input": false
        });

        let result = tool.execute(params).await.unwrap();
        assert!(!result["streaming"].as_bool().unwrap());
        assert!(result["stdout"].as_str().unwrap().contains("hello world"));
    }

    #[tokio::test]
    async fn test_streaming_mode() {
        let tool = StreamExecTool::new();
        assert_eq!(tool.streaming_mode(), StreamingMode::Both);

        let params = serde_json::json!({
            "command": "cat",
            "args": [],
            "stream_output": true,
            "stream_input": true
        });

        let result = tool.execute(params).await.unwrap();
        assert!(result["streaming"].as_bool().unwrap());
        assert_eq!(result["mode"], "both");
    }

    #[tokio::test]
    async fn test_streaming_echo() {
        let tool = StreamExecTool::new();
        let (resp_tx, mut resp_rx) = mpsc::channel(10);

        // Create initial request with command
        let params = StreamExecParams {
            command: "echo".to_string(),
            args: vec!["test output".to_string()],
            stream_output: true,
            stream_input: false,
        };

        let first_request = StreamRequest {
            id: 1,
            sequence: 0,
            data: serde_json::to_vec(&params).unwrap(),
            is_last: true,
        };

        let input = Box::pin(stream::iter(vec![first_request]));

        // Process stream
        tokio::spawn(async move {
            tool.process_stream(input, resp_tx).await.unwrap();
        });

        // Collect responses
        let mut responses = Vec::new();
        while let Some(resp) = resp_rx.recv().await {
            if resp.is_last {
                break;
            }
            responses.push(resp);
        }

        // Should have received the output
        assert!(!responses.is_empty());
        let output = String::from_utf8(responses[0].data.to_vec()).unwrap();
        assert!(output.contains("test output"));
    }
}
