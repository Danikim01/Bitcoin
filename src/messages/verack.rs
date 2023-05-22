use crate::messages::constants::commands::VERACK;
use crate::messages::{Message, MessageHeader};
use std::io;
use std::net::TcpStream;

#[derive(Debug)]
pub struct VerAck {
    _message_header: MessageHeader,
}

impl VerAck {
    pub fn new() -> Self {
        Self {
            _message_header: MessageHeader::default(),
        }
    }

    pub fn from_stream(stream: &mut TcpStream) -> Result<Self, io::Error> {
        let message_header = MessageHeader::from_stream(stream)?;
        Ok(VerAck { _message_header: message_header })
    }
}

impl Message for VerAck {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let message = self.build_message(VERACK, None)?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_header(){
        let header:Vec<u8> = vec![
            0xF9, 0xBE, 0xB4, 0xD9, 0x76, 0x65, 0x72, 0x61, 0x63, 0x6B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,0x00,0x5D, 0xF6, 0xE0, 0xE2
        ];
        let message_header = MessageHeader::from_bytes(&header).unwrap();
        assert_eq!(message_header.start_string,[0xF9,0xBE,0xB4,0xD9]);
        assert_eq!(message_header.command_name,"verack\0\0\0\0\0\0".to_string());
        assert_eq!(message_header.payload_size,0);
        assert_eq!(message_header.checksum,[0x5D,0xF6,0xE0,0xE2]);
    }

  

    
}
