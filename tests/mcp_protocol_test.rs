// Tests for MCP protocol to improve coverage
use pcode::mcp::{
    protocol::{Message, ProtocolHandler},
    McpError, McpProtocol,
};

#[tokio::test]
async fn test_mcp_protocol_creation() {
    let _protocol = McpProtocol::new();

    // Just verify it creates successfully
}

#[test]
fn test_protocol_handler_invalid_data() {
    let handler = ProtocolHandler::new();

    // Test decoding invalid data - too short (less than 12 bytes)
    let invalid_data = vec![0xFF, 0xFF]; // Too short
    let result = handler.decode_message(&invalid_data);
    assert!(result.is_err());

    // Test decoding with incomplete payload
    let mut incomplete = vec![0u8; 8]; // ID bytes
    incomplete.extend_from_slice(&[10, 0, 0, 0]); // Length = 10
                                                  // But no payload data follows
    let result = handler.decode_message(&incomplete);
    assert!(result.is_err());
}

#[test]
fn test_message_serialization() {
    let handler = ProtocolHandler::new();

    // Test various message sizes
    let test_cases = vec![
        Message {
            id: 0,
            payload: vec![],
        },
        Message {
            id: 1,
            payload: vec![0x42],
        },
        Message {
            id: u64::MAX,
            payload: vec![0xFF; 100],
        },
        Message {
            id: 12345,
            payload: (0..255).collect(),
        },
    ];

    for original in test_cases {
        let encoded = handler.encode_message(&original).unwrap();
        let decoded = handler.decode_message(&encoded).unwrap();

        assert_eq!(original.id, decoded.id);
        assert_eq!(original.payload, decoded.payload);
    }
}

#[test]
fn test_mcp_error_conversions() {
    // Test From<std::io::Error> for McpError
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
    let mcp_err: McpError = McpError::Transport(io_err.to_string());
    assert!(mcp_err.to_string().contains("Transport error"));

    // Test error chaining
    let err = McpError::Protocol("bad message".to_string());
    let pcode_err: pcode::PcodeError = err.into();
    assert!(pcode_err.to_string().contains("MCP protocol error"));
}

#[test]
fn test_message_debug_impl() {
    let msg = Message {
        id: 42,
        payload: vec![1, 2, 3],
    };

    let debug_str = format!("{:?}", msg);
    assert!(debug_str.contains("42"));
    assert!(debug_str.contains("payload"));
}

#[test]
fn test_protocol_handler_edge_cases() {
    let handler = ProtocolHandler::new();

    // Empty payload
    let msg = Message {
        id: 0,
        payload: vec![],
    };
    let encoded = handler.encode_message(&msg).unwrap();
    assert_eq!(encoded.len(), 12); // 8 bytes ID + 4 bytes length

    // Large payload
    let large_payload = vec![0xAB; 10_000];
    let msg = Message {
        id: 999,
        payload: large_payload,
    };
    let encoded = handler.encode_message(&msg).unwrap();
    let decoded = handler.decode_message(&encoded).unwrap();
    assert_eq!(decoded.payload.len(), 10_000);
}

#[test]
fn test_protocol_decode_partial_message() {
    let handler = ProtocolHandler::new();
    let msg = Message {
        id: 123,
        payload: vec![1, 2, 3, 4, 5],
    };
    let encoded = handler.encode_message(&msg).unwrap();

    // Try to decode with partial data
    let partial = &encoded[..encoded.len() - 2];
    let result = handler.decode_message(partial);
    assert!(result.is_err());
}
