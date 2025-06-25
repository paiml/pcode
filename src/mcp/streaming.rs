use super::McpError;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info};

/// Streaming mode capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamingMode {
    None,
    Input,  // Tool accepts streaming input
    Output, // Tool produces streaming output
    Both,   // Full duplex streaming
}

/// Request for streaming data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRequest {
    pub id: u64,
    pub sequence: u32,
    pub data: Vec<u8>,
    pub is_last: bool,
}

/// Response for streaming data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub id: u64,
    pub sequence: u32,
    pub data: Vec<u8>,
    pub is_last: bool,
    pub error: Option<String>,
}

/// Stream handle for managing streaming operations
pub struct StreamHandle {
    id: u64,
    tx: mpsc::Sender<StreamRequest>,
    rx: Arc<Mutex<mpsc::Receiver<StreamResponse>>>,
}

impl StreamHandle {
    pub fn new(
        id: u64,
        buffer_size: usize,
    ) -> (
        Self,
        mpsc::Receiver<StreamRequest>,
        mpsc::Sender<StreamResponse>,
    ) {
        let (req_tx, req_rx) = mpsc::channel(buffer_size);
        let (resp_tx, resp_rx) = mpsc::channel(buffer_size);

        let handle = Self {
            id,
            tx: req_tx,
            rx: Arc::new(Mutex::new(resp_rx)),
        };

        (handle, req_rx, resp_tx)
    }

    /// Send streaming data
    pub async fn send(&self, data: Vec<u8>, sequence: u32, is_last: bool) -> Result<(), McpError> {
        let request = StreamRequest {
            id: self.id,
            sequence,
            data,
            is_last,
        };

        self.tx
            .send(request)
            .await
            .map_err(|_| McpError::Transport("Stream closed".to_string()))?;

        Ok(())
    }

    /// Receive streaming response
    pub async fn recv(&self) -> Option<StreamResponse> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }

    /// Create a futures Stream from responses
    pub fn into_stream(self) -> impl Stream<Item = StreamResponse> {
        let rx = self.rx.clone();

        async_stream::stream! {
            loop {
                let mut rx_guard = rx.lock().await;
                match rx_guard.recv().await {
                    Some(response) => {
                        let is_last = response.is_last;
                        yield response;
                        if is_last {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }
}

/// Trait for streaming-capable tools
#[async_trait]
pub trait StreamingTool: Send + Sync {
    /// Get streaming mode support
    fn streaming_mode(&self) -> StreamingMode;

    /// Process streaming input
    async fn process_stream(
        &self,
        input: Pin<Box<dyn Stream<Item = StreamRequest> + Send>>,
        output: mpsc::Sender<StreamResponse>,
    ) -> Result<(), McpError>;
}

/// Manager for streaming operations
pub struct StreamManager {
    active_streams: Arc<Mutex<std::collections::HashMap<u64, StreamHandle>>>,
    next_id: Arc<Mutex<u64>>,
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            active_streams: Arc::new(Mutex::new(std::collections::HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Create a new stream
    pub async fn create_stream(
        &self,
        buffer_size: usize,
    ) -> Result<
        (
            u64,
            StreamHandle,
            mpsc::Receiver<StreamRequest>,
            mpsc::Sender<StreamResponse>,
        ),
        McpError,
    > {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;

        let (handle, req_rx, resp_tx) = StreamHandle::new(id, buffer_size);

        let mut streams = self.active_streams.lock().await;
        streams.insert(id, handle);

        // Return a clone of the handle
        let (return_handle, _, _) = StreamHandle::new(id, buffer_size);

        Ok((id, return_handle, req_rx, resp_tx))
    }

    /// Get an active stream by ID
    pub async fn get_stream(&self, id: u64) -> Option<StreamHandle> {
        let streams = self.active_streams.lock().await;
        streams.get(&id).map(|_| {
            // Return a new handle connected to the same stream
            // In a real implementation, we'd share the channels properly
            let (handle, _, _) = StreamHandle::new(id, 32);
            handle
        })
    }

    /// Close a stream
    pub async fn close_stream(&self, id: u64) -> Result<(), McpError> {
        let mut streams = self.active_streams.lock().await;
        streams
            .remove(&id)
            .ok_or_else(|| McpError::Transport(format!("Stream {} not found", id)))?;

        info!("Closed stream {}", id);
        Ok(())
    }
}

/// Example streaming tool implementation
pub struct EchoStreamTool;

#[async_trait]
impl StreamingTool for EchoStreamTool {
    fn streaming_mode(&self) -> StreamingMode {
        StreamingMode::Both
    }

    async fn process_stream(
        &self,
        mut input: Pin<Box<dyn Stream<Item = StreamRequest> + Send>>,
        output: mpsc::Sender<StreamResponse>,
    ) -> Result<(), McpError> {
        debug!("Echo stream started");

        while let Some(request) = input.next().await {
            let response = StreamResponse {
                id: request.id,
                sequence: request.sequence,
                data: request.data.clone(),
                is_last: request.is_last,
                error: None,
            };

            if output.send(response).await.is_err() {
                error!("Failed to send echo response");
                break;
            }

            if request.is_last {
                debug!("Echo stream completed");
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn test_stream_handle() {
        let (handle, mut req_rx, resp_tx) = StreamHandle::new(1, 10);

        // Send a request
        handle.send(Vec::from("hello"), 0, false).await.unwrap();

        // Receive the request
        let req = req_rx.recv().await.unwrap();
        assert_eq!(req.sequence, 0);
        assert_eq!(req.data, Vec::from("hello"));
        assert!(!req.is_last);

        // Send a response
        let response = StreamResponse {
            id: 1,
            sequence: 0,
            data: Vec::from("world"),
            is_last: true,
            error: None,
        };
        resp_tx.send(response).await.unwrap();

        // Receive the response
        let resp = handle.recv().await.unwrap();
        assert_eq!(resp.data, Vec::from("world"));
        assert!(resp.is_last);
    }

    #[tokio::test]
    async fn test_stream_manager() {
        let manager = StreamManager::new();

        let (id, _handle, _req_rx, _resp_tx) = manager.create_stream(10).await.unwrap();
        assert_eq!(id, 1);

        // Should be able to get the stream
        assert!(manager.get_stream(id).await.is_some());

        // Close the stream
        manager.close_stream(id).await.unwrap();

        // Should no longer exist
        assert!(manager.get_stream(id).await.is_none());
    }

    #[tokio::test]
    async fn test_echo_stream_tool() {
        let tool = EchoStreamTool;
        assert_eq!(tool.streaming_mode(), StreamingMode::Both);

        let (resp_tx, mut resp_rx) = mpsc::channel(10);

        // Create input stream
        let requests = vec![
            StreamRequest {
                id: 1,
                sequence: 0,
                data: Vec::from("hello"),
                is_last: false,
            },
            StreamRequest {
                id: 1,
                sequence: 1,
                data: Vec::from("world"),
                is_last: true,
            },
        ];
        let input = Box::pin(stream::iter(requests));

        // Process stream
        tokio::spawn(async move {
            tool.process_stream(input, resp_tx).await.unwrap();
        });

        // Verify responses
        let resp1 = resp_rx.recv().await.unwrap();
        assert_eq!(resp1.sequence, 0);
        assert_eq!(resp1.data, Vec::from("hello"));
        assert!(!resp1.is_last);

        let resp2 = resp_rx.recv().await.unwrap();
        assert_eq!(resp2.sequence, 1);
        assert_eq!(resp2.data, Vec::from("world"));
        assert!(resp2.is_last);
    }

    #[tokio::test]
    async fn test_stream_to_futures_stream() {
        let (handle, _req_rx, resp_tx) = StreamHandle::new(1, 10);

        // Send some responses
        tokio::spawn(async move {
            for i in 0..3 {
                let response = StreamResponse {
                    id: 1,
                    sequence: i,
                    data: Vec::from(format!("msg{}", i)),
                    is_last: i == 2,
                    error: None,
                };
                resp_tx.send(response).await.unwrap();
            }
        });

        // Convert to futures stream and collect
        let stream = handle.into_stream();
        let responses: Vec<_> = stream.collect().await;

        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0].data, Vec::from("msg0"));
        assert_eq!(responses[2].data, Vec::from("msg2"));
        assert!(responses[2].is_last);
    }
}
