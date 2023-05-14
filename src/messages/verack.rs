use crate::messages::constants::commands::VERACK;
use crate::messages::{Message, MessageHeader};
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

    pub fn from_stream(stream: &mut TcpStream) -> Result<Self, io::Error> {
        let message_header = MessageHeader::from_stream(stream)?;
        Ok(VerAck { message_header })
    }
}

impl Message for VerAck {
    fn send_to(&self, stream: &mut TcpStream) -> io::Result<()> {
        let message = self.build_message(VERACK, None)?;
        stream.write_all(&message)?;
        stream.flush()
    }
}
