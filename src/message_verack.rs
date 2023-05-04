use crate::message_header;
use crate::message_header::MessageHeader;
use crate::message_version::Version;
use crate::messages::Message;
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use std::io::Cursor;
use std::io::{Read, Write};
use std::net::TcpStream;
#[derive(Debug)]
pub struct VerAckMessage {
    message_header: MessageHeader,
}

impl VerAckMessage {
    pub fn new() -> VerAckMessage {
        VerAckMessage {
            message_header: MessageHeader::default(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<VerAckMessage, String> {
        let mut cursor = Cursor::new(bytes);

        // verack only has header
        let mut messageHeaderBytes = [0_u8; 24];
        cursor
            .read_exact(&mut messageHeaderBytes)
            .map_err(|error| error.to_string())?;
        let message_header =
            MessageHeader::from_bytes(&messageHeaderBytes).map_err(|error| error.to_string())?;

        Ok(VerAckMessage { message_header })
    }
}

impl Message for VerAckMessage {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let message = self.build_message("verack".to_string(), None)?;
        stream.write_all(&message)?;
        stream.flush()?;
        Ok(())
    }
}
