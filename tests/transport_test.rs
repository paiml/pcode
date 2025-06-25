use pcode::mcp::protocol::{Message, ProtocolHandler};
use pcode::mcp::transport::StdioTransport;

#[test]
fn test_protocol_handler_creation() {
    let _handler = ProtocolHandler::new();
    let _handler2 = ProtocolHandler::default();
    // Just verify they create successfully
}

#[test]
fn test_message_encoding_empty() {
    let handler = ProtocolHandler::new();
    let message = Message {
        id: 0,
        payload: vec![],
    };

    let encoded = handler.encode_message(&message).unwrap();
    assert_eq!(encoded.len(), 12); // 8 bytes id + 4 bytes length + 0 payload
}

#[test]
fn test_message_decode_too_short() {
    let handler = ProtocolHandler::new();
    let data = vec![1, 2, 3]; // Too short

    let result = handler.decode_message(&data);
    assert!(result.is_err());
}

#[test]
fn test_message_decode_incomplete() {
    let handler = ProtocolHandler::new();
    // Valid header but incomplete payload
    let mut data = vec![0; 12];
    data[8..12].copy_from_slice(&10u32.to_le_bytes()); // Says 10 bytes but no payload

    let result = handler.decode_message(&data);
    assert!(result.is_err());
}

#[test]
fn test_stdio_transport_creation() {
    let _transport = StdioTransport::new();
    let _transport2 = StdioTransport::default();
    // Just verify they create successfully
}
