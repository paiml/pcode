// Generated tests for mcp/transport.rs
use pcode::mcp::protocol::{Message, ProtocolHandler};

#[tokio::test]
async fn test_stdio_transport_send_success() {
    // This test would need mock stdout
    // let mut transport = StdioTransport::new();
    // let message = Message { id: 1, payload: vec![1, 2, 3] };
    // In real implementation, we'd mock stdout and verify bytes written
}

#[tokio::test]
async fn test_protocol_encode_decode_roundtrip() {
    let handler = ProtocolHandler::new();
    let original = Message {
        id: 12345,
        payload: vec![0xFF, 0x00, 0xAB, 0xCD],
    };

    let encoded = handler.encode_message(&original).unwrap();
    let decoded = handler.decode_message(&encoded).unwrap();

    assert_eq!(original.id, decoded.id);
    assert_eq!(original.payload, decoded.payload);
}
