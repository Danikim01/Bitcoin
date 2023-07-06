use super::utility::read_from_varint;
use crate::messages::utility::read_hash;
use crate::messages::utility::StreamRead;
use crate::messages::{constants::commands::GETHEADERS, utility::to_varint, HashId, Serialize};
use crate::messages::{
    constants::{commands, config::VERBOSE},
    Block, GetData, Headers, Message, MessageHeader, Ping, VerAck, Version,
};
use std::io::Cursor;

/// Struct that represents the data GetHeader message
#[derive(Debug, Clone, PartialEq)]
pub struct GetHeader {
    pub version: u32,
    pub hash_count: u8,
    pub block_header_hashes: Vec<HashId>,
    pub stop_hash: HashId,
}

impl GetHeader {
    fn build_payload(&self) -> std::io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        payload.extend(&self.version.to_le_bytes());
        let hash_count_a_enviar = to_varint(self.hash_count as u64);
        payload.extend(&hash_count_a_enviar);
        //payload.extend(&self.hash_count.to_le_bytes());
        for header_hash in &self.block_header_hashes {
            payload.extend(header_hash.iter());
        }
        payload.extend(self.stop_hash.iter());
        Ok(payload)
    }

    /// Create a new getheaders from a last header hash
    pub fn from_last_header(last_header: HashId) -> Self {
        Self {
            version: 70015,
            hash_count: 1,
            block_header_hashes: vec![last_header],
            stop_hash: HashId::default(),
        }
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Message, std::io::Error> {
        let mut cursor = Cursor::new(bytes);
        let version = u32::from_le_stream(&mut cursor)?;
        let hash_count = read_from_varint(&mut cursor)?;
        let mut block_headers_hashes = vec![];
        for _ in 0..hash_count {
            let hash = read_hash(&mut cursor)?;
            block_headers_hashes.push(hash);
        }
        let stop_hash = read_hash(&mut cursor)?;

        let get_header = GetHeader {
            version,
            hash_count: hash_count as u8,
            block_header_hashes: block_headers_hashes,
            stop_hash: stop_hash,
        };

        println!("GetHeader received: {:?}", get_header);

        Ok(Message::_GetHeader(get_header))
    }
}

impl Serialize for GetHeader {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let payload = self.build_payload()?;
        let message = self.build_message(GETHEADERS, Some(payload))?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::utility::_decode_hex;
    #[test]
    fn deserialize_getheaders_message() {
        let bytes = _decode_hex("7111010002d39f608a7775b537729884d4e6633bb2105e55a16a14d31b00000000000000005c3e6403d40837110a2e8afb602b1c01714bda7ce23bea0a00000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let getheaders_message = GetHeader::deserialize(&bytes.unwrap()).unwrap();

        if let Message::_GetHeader(getheaders) = getheaders_message {
            // Acceder a Headers dentro de Message::Headers
            // Utilizar el valor `headers` aquí
            assert_eq!(getheaders.version, 70001);
            assert_eq!(getheaders.hash_count, 2);

            let hash1 = HashId {
                hash: [
                    0xd3, 0x9f, 0x60, 0x8a, 0x77, 0x75, 0xb5, 0x37, 0x72, 0x98, 0x84, 0xd4, 0xe6,
                    0x63, 0x3b, 0xb2, 0x10, 0x5e, 0x55, 0xa1, 0x6a, 0x14, 0xd3, 0x1b, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
            };

            let hash2 = HashId {
                hash: [
                    0x5c, 0x3e, 0x64, 0x03, 0xd4, 0x08, 0x37, 0x11, 0x0a, 0x2e, 0x8a, 0xfb, 0x60,
                    0x2b, 0x1c, 0x01, 0x71, 0x4b, 0xda, 0x7c, 0xe2, 0x3b, 0xea, 0x0a, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
            };

            assert_eq!(getheaders.block_header_hashes[0], hash1);
            assert_eq!(getheaders.block_header_hashes[1], hash2);

            let stop_hash = HashId { hash: [0; 32] };

            assert_eq!(getheaders.stop_hash, stop_hash);
        }
    }
}
