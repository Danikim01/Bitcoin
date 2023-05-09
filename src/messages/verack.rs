use crate::messages::{MessageHeader, Message};
use std::io::Cursor;
use std::io::{self, Read, Write};
use std::net::TcpStream;
#[derive(Debug)]
pub struct VerAck {
    message_header: MessageHeader,
}

impl VerAck {
    pub fn new() -> Self {
        Self {
            message_header: MessageHeader::default(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, io::Error> {
        let mut cursor = Cursor::new(bytes);

        // verack only has header
        let mut message_header_bytes = [0_u8; 24];
        cursor.read_exact(&mut message_header_bytes)?;
        let message_header = MessageHeader::from_bytes(&message_header_bytes)?;

        Ok(VerAck { message_header })
    }
}

impl Message for VerAck {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let message = self.build_message("verack", None)?;
        stream.write_all(&message)?;
        stream.flush()
    }
}
