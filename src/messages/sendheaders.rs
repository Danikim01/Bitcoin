use crate::messages::constants::commands::SENDHEADERS;
use crate::messages::{MessageHeader, Serialize};
use std::io;
use std::net::TcpStream;

/// Struct that represents the SendHeaders message
#[derive(Debug, Clone)]
pub struct SendHeaders {
    _message_header: MessageHeader,
}

impl SendHeaders {
    /// Creates a new `SendHeaders` message with the default values.
    pub fn new() -> Self {
        Self {
            _message_header: MessageHeader::default(),
        }
    }

    /// Reads the data from the stream and returns a `SendHeaders` message.
    pub fn _from_stream(stream: &mut TcpStream) -> Result<Self, io::Error> {
        let message_header = MessageHeader::from_stream(stream)?;
        Ok(SendHeaders {
            _message_header: message_header,
        })
    }
}

impl Serialize for SendHeaders {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let message = self.build_message(SENDHEADERS, None)?;
        Ok(message)
    }
}
