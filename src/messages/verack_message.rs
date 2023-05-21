use crate::messages::constants::commands::VERACK;
use crate::messages::{MessageHeader, Serialize};
use std::io;
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

    pub fn from_stream(stream: &mut TcpStream) -> Result<Self, io::Error> {
        let message_header = MessageHeader::from_stream(stream)?;
        Ok(VerAck { message_header })
    }
}

impl Serialize for VerAck {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let message = self.build_message(VERACK, None)?;
        Ok(message)
    }
}
