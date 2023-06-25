use crate::messages::{
    constants::{commands::PONG, config::MAGIC},
    utility::StreamRead,
    Message, MessageHeader, Serialize,
};
use crate::utility::double_hash;
use std::io::{self, Cursor};

#[derive(Debug, Clone)]
pub struct Ping {
    pub nonce: u64,
}

impl Ping {
    fn new(nonce: u64) -> Self {
        Self { nonce }
    }

    pub fn pong(bytes: &[u8]) -> io::Result<Vec<u8>> {
        let hash = double_hash(bytes);
        let checksum: [u8; 4] = [hash[0], hash[1], hash[2], hash[3]];
        let message_header =
            MessageHeader::new(MAGIC, PONG.to_string(), bytes.len() as u32, checksum);
        let mut payload = Vec::new();
        payload.extend(&message_header.serialize()?);
        payload.extend(bytes);
        Ok(payload)
    }
}

impl Serialize for Ping {
    fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend(self.nonce.to_le_bytes());
        Ok(bytes)
    }

    fn deserialize(bytes: &[u8]) -> io::Result<Message> {
        let mut cursor = Cursor::new(bytes);
        let nonce = u64::from_le_stream(&mut cursor)?;
        Ok(Message::Ping(Self::new(nonce)))
    }
}
