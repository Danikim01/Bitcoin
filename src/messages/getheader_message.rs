use crate::messages::{utility::to_varint, HashId, Serialize};

#[derive(Debug)]
pub struct GetHeader {
    version: i32,
    hash_count: u8,
    block_header_hashes: Vec<HashId>,
    stop_hash: HashId,
}

//default for genesis block
impl Default for GetHeader {
    fn default() -> Self {
        Self::new(
            70015,
            1,
            vec![[
                0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63,
                0xf7, 0x4f, 0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ]], //genesis hash
            [0_u8; 32], //til max block hashes (500 is MAX for response)
        )
    }
}

impl GetHeader {
    fn new(
        version: i32,
        hash_count: u8,
        block_header_hashes: Vec<[u8; 32]>,
        stop_hash: [u8; 32],
    ) -> Self {
        Self {
            version,
            hash_count,
            block_header_hashes,
            stop_hash,
        }
    }

    fn build_payload(&self) -> std::io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        payload.extend(&self.version.to_le_bytes());
        let hash_count_a_enviar = to_varint(self.hash_count as u64);
        payload.extend(&hash_count_a_enviar);
        //payload.extend(&self.hash_count.to_le_bytes());
        for header_hash in &self.block_header_hashes {
            payload.extend(header_hash);
        }
        payload.extend(self.stop_hash);
        Ok(payload)
    }

    pub fn from_last_header(last_header: HashId) -> Self {
        Self {
            version: 70015,
            hash_count: 1,
            block_header_hashes: vec![last_header],
            stop_hash: [0u8; 32],
        }
    }
}

impl Serialize for GetHeader {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let payload = self.build_payload()?;
        let message = self.build_message("getheaders", Some(payload))?;
        Ok(message)
    }
}
