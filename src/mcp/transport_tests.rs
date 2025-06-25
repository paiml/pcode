#[cfg(test)]
mod transport_async_tests {
    use crate::mcp::protocol::Message;
    use crate::mcp::transport::*;
    use std::io;

    // Mock transport for testing
    struct MockTransport {
        sent_messages: Vec<Message>,
        receive_messages: Vec<Message>,
        receive_index: usize,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                sent_messages: vec![],
                receive_messages: vec![],
                receive_index: 0,
            }
        }

        fn add_message_to_receive(&mut self, msg: Message) {
            self.receive_messages.push(msg);
        }
    }

    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn send(&mut self, message: Message) -> Result<(), io::Error> {
            self.sent_messages.push(message);
            Ok(())
        }

        async fn receive(&mut self) -> Result<Message, io::Error> {
            if self.receive_index < self.receive_messages.len() {
                let msg = self.receive_messages[self.receive_index].clone();
                self.receive_index += 1;
                Ok(msg)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "No more messages",
                ))
            }
        }
    }

    #[tokio::test]
    async fn test_mock_transport_send() {
        let mut transport = MockTransport::new();
        let msg = Message {
            id: 1,
            payload: vec![1, 2, 3],
        };

        transport.send(msg.clone()).await.unwrap();
        assert_eq!(transport.sent_messages.len(), 1);
        assert_eq!(transport.sent_messages[0].id, 1);
    }

    #[tokio::test]
    async fn test_mock_transport_receive() {
        let mut transport = MockTransport::new();
        let msg = Message {
            id: 2,
            payload: vec![4, 5, 6],
        };
        transport.add_message_to_receive(msg.clone());

        let received = transport.receive().await.unwrap();
        assert_eq!(received.id, 2);
        assert_eq!(received.payload, vec![4, 5, 6]);
    }
}
