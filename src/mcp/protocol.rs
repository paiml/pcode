use capnp::message::ReaderOptions;
use serde::{Deserialize, Serialize};
use std::io::Write;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub payload: Vec<u8>,
}

pub struct ProtocolHandler {
    _reader_options: ReaderOptions,
}

impl Default for ProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolHandler {
    pub fn new() -> Self {
        Self {
            _reader_options: ReaderOptions {
                traversal_limit_in_words: Some(8 * 1024 * 1024), // 64MB limit
                nesting_limit: 64,
            },
        }
    }

    pub fn encode_message(&self, message: &Message) -> Result<Vec<u8>, capnp::Error> {
        // In real implementation, would use Cap'n Proto schema
        // For now, using simple encoding
        let mut output = Vec::new();
        output.write_all(&message.id.to_le_bytes())?;
        output.write_all(&(message.payload.len() as u32).to_le_bytes())?;
        output.write_all(&message.payload)?;

        debug!("Encoded message {} with {} bytes", message.id, output.len());
        Ok(output)
    }

    pub fn decode_message(&self, data: &[u8]) -> Result<Message, capnp::Error> {
        if data.len() < 12 {
            return Err(capnp::Error::failed("Message too short".to_string()));
        }

        let id = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let len = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

        if data.len() < 12 + len {
            return Err(capnp::Error::failed("Incomplete message".to_string()));
        }

        let payload = data[12..12 + len].to_vec();

        debug!("Decoded message {} with {} bytes", id, payload.len());
        Ok(Message { id, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_encoding() {
        let handler = ProtocolHandler::new();
        let message = Message {
            id: 42,
            payload: vec![1, 2, 3, 4, 5],
        };

        let encoded = handler.encode_message(&message).unwrap();
        let decoded = handler.decode_message(&encoded).unwrap();

        assert_eq!(decoded.id, message.id);
        assert_eq!(decoded.payload, message.payload);
    }
}
