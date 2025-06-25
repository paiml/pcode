use super::streaming::{StreamRequest, StreamResponse, StreamingMode};
use capnp::message::ReaderOptions;
use serde::{Deserialize, Serialize};
use std::io::Write;
use tracing::debug;

/// Message types for MCP protocol v2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageType {
    /// Regular request/response
    Request {
        method: String,
        params: serde_json::Value,
    },
    Response {
        result: Option<serde_json::Value>,
        error: Option<String>,
    },
    /// Streaming messages
    StreamInit {
        method: String,
        params: serde_json::Value,
        mode: StreamingMode,
    },
    StreamData(StreamRequest),
    StreamResponse(StreamResponse),
    StreamClose {
        stream_id: u64,
        reason: Option<String>,
    },
}

/// Enhanced message with streaming support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageV2 {
    pub id: u64,
    pub message_type: MessageType,
    pub correlation_id: Option<u64>, // For linking stream messages
}

/// Enhanced protocol handler with streaming support
pub struct ProtocolHandlerV2 {
    _reader_options: ReaderOptions,
}

impl Default for ProtocolHandlerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolHandlerV2 {
    pub fn new() -> Self {
        Self {
            _reader_options: ReaderOptions {
                traversal_limit_in_words: Some(8 * 1024 * 1024), // 64MB limit
                nesting_limit: 64,
            },
        }
    }

    /// Encode a message for transmission
    pub fn encode_message(
        &self,
        message: &MessageV2,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Serialize to JSON first
        let json_bytes = serde_json::to_vec(message)?;

        // Create length-prefixed message
        let mut output = Vec::with_capacity(4 + json_bytes.len());
        output.write_all(&(json_bytes.len() as u32).to_le_bytes())?;
        output.write_all(&json_bytes)?;

        debug!(
            "Encoded message {} type {:?} ({} bytes)",
            message.id,
            std::mem::discriminant(&message.message_type),
            output.len()
        );

        Ok(output)
    }

    /// Decode a message from bytes
    pub fn decode_message(&self, data: &[u8]) -> Result<MessageV2, Box<dyn std::error::Error>> {
        if data.len() < 4 {
            return Err("Message too short".into());
        }

        let len = u32::from_le_bytes(data[0..4].try_into()?) as usize;

        if data.len() < 4 + len {
            return Err("Incomplete message".into());
        }

        let message: MessageV2 = serde_json::from_slice(&data[4..4 + len])?;

        debug!(
            "Decoded message {} type {:?}",
            message.id,
            std::mem::discriminant(&message.message_type)
        );

        Ok(message)
    }

    /// Check if a message requires streaming
    pub fn is_streaming_message(&self, message: &MessageV2) -> bool {
        matches!(
            message.message_type,
            MessageType::StreamInit { .. }
                | MessageType::StreamData(_)
                | MessageType::StreamResponse(_)
                | MessageType::StreamClose { .. }
        )
    }

    /// Create a stream initialization message
    pub fn create_stream_init(
        &self,
        id: u64,
        method: String,
        params: serde_json::Value,
        mode: StreamingMode,
    ) -> MessageV2 {
        MessageV2 {
            id,
            message_type: MessageType::StreamInit {
                method,
                params,
                mode,
            },
            correlation_id: None,
        }
    }

    /// Create a stream data message
    pub fn create_stream_data(&self, id: u64, request: StreamRequest) -> MessageV2 {
        let correlation_id = request.id;
        MessageV2 {
            id,
            message_type: MessageType::StreamData(request),
            correlation_id: Some(correlation_id),
        }
    }

    /// Create a stream response message
    pub fn create_stream_response(&self, id: u64, response: StreamResponse) -> MessageV2 {
        let correlation_id = response.id;
        MessageV2 {
            id,
            message_type: MessageType::StreamResponse(response),
            correlation_id: Some(correlation_id),
        }
    }

    /// Create a stream close message
    pub fn create_stream_close(
        &self,
        id: u64,
        stream_id: u64,
        reason: Option<String>,
    ) -> MessageV2 {
        MessageV2 {
            id,
            message_type: MessageType::StreamClose { stream_id, reason },
            correlation_id: Some(stream_id),
        }
    }
}

/// Frame decoder for reading messages from a stream
pub struct FrameDecoder {
    buffer: Vec<u8>,
    expected_len: Option<usize>,
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            expected_len: None,
        }
    }

    /// Feed data into the decoder
    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Try to decode a message
    pub fn try_decode(&mut self) -> Option<Result<MessageV2, Box<dyn std::error::Error>>> {
        // First, try to read the length
        if self.expected_len.is_none() && self.buffer.len() >= 4 {
            let len = u32::from_le_bytes(self.buffer[0..4].try_into().unwrap()) as usize;
            self.expected_len = Some(len);
        }

        // Then try to read the message
        if let Some(expected) = self.expected_len {
            if self.buffer.len() >= 4 + expected {
                // Extract the message
                let message_data = self.buffer[..4 + expected].to_vec();
                self.buffer.drain(..4 + expected);
                self.expected_len = None;

                // Decode
                let handler = ProtocolHandlerV2::new();
                return Some(handler.decode_message(&message_data));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_v2_encoding() {
        let handler = ProtocolHandlerV2::new();
        let message = MessageV2 {
            id: 123,
            message_type: MessageType::Request {
                method: "test".to_string(),
                params: serde_json::json!({"key": "value"}),
            },
            correlation_id: None,
        };

        let encoded = handler.encode_message(&message).unwrap();
        let decoded = handler.decode_message(&encoded).unwrap();

        assert_eq!(decoded.id, message.id);
        match decoded.message_type {
            MessageType::Request { method, params } => {
                assert_eq!(method, "test");
                assert_eq!(params["key"], "value");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_streaming_messages() {
        let handler = ProtocolHandlerV2::new();

        // Test stream init
        let init = handler.create_stream_init(
            1,
            "process".to_string(),
            serde_json::json!({"cmd": "ls"}),
            StreamingMode::Output,
        );
        assert!(handler.is_streaming_message(&init));

        // Test stream data
        let request = StreamRequest {
            id: 100,
            sequence: 0,
            data: b"test data".to_vec(),
            is_last: false,
        };
        let data_msg = handler.create_stream_data(2, request);
        assert!(handler.is_streaming_message(&data_msg));
        assert_eq!(data_msg.correlation_id, Some(100));

        // Test stream close
        let close = handler.create_stream_close(3, 100, Some("completed".to_string()));
        assert!(handler.is_streaming_message(&close));
        assert_eq!(close.correlation_id, Some(100));
    }

    #[test]
    fn test_frame_decoder() {
        let handler = ProtocolHandlerV2::new();
        let message = MessageV2 {
            id: 456,
            message_type: MessageType::Response {
                result: Some(serde_json::json!({"status": "ok"})),
                error: None,
            },
            correlation_id: None,
        };

        let encoded = handler.encode_message(&message).unwrap();

        // Test feeding data in chunks
        let mut decoder = FrameDecoder::new();

        // Feed first half
        decoder.feed(&encoded[..encoded.len() / 2]);
        assert!(decoder.try_decode().is_none());

        // Feed second half
        decoder.feed(&encoded[encoded.len() / 2..]);
        let decoded = decoder.try_decode().unwrap().unwrap();

        assert_eq!(decoded.id, 456);
        match decoded.message_type {
            MessageType::Response { result, error } => {
                assert!(result.is_some());
                assert!(error.is_none());
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_multiple_messages_in_buffer() {
        let handler = ProtocolHandlerV2::new();
        let mut decoder = FrameDecoder::new();

        // Create two messages
        let msg1 = MessageV2 {
            id: 1,
            message_type: MessageType::Request {
                method: "first".to_string(),
                params: serde_json::json!({}),
            },
            correlation_id: None,
        };

        let msg2 = MessageV2 {
            id: 2,
            message_type: MessageType::Request {
                method: "second".to_string(),
                params: serde_json::json!({}),
            },
            correlation_id: None,
        };

        // Encode and feed both
        let encoded1 = handler.encode_message(&msg1).unwrap();
        let encoded2 = handler.encode_message(&msg2).unwrap();

        decoder.feed(&encoded1);
        decoder.feed(&encoded2);

        // Should be able to decode both
        let decoded1 = decoder.try_decode().unwrap().unwrap();
        assert_eq!(decoded1.id, 1);

        let decoded2 = decoder.try_decode().unwrap().unwrap();
        assert_eq!(decoded2.id, 2);

        // No more messages
        assert!(decoder.try_decode().is_none());
    }
}
