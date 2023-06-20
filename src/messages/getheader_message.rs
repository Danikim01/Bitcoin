use crate::messages::{constants::commands::GETHEADERS, constants::messages::GENESIS_HASHID, utility::to_varint, HashId, Serialize};

#[derive(Debug, Clone)]
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
            vec![GENESIS_HASHID],
            HashId::default(), //til max block hashes (500 is MAX for response)
        )
    }
}

impl GetHeader {
    fn new(
        version: i32,
        hash_count: u8,
        block_header_hashes: Vec<HashId>,
        stop_hash: HashId,
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
            payload.extend(header_hash.iter());
        }
        payload.extend(self.stop_hash.iter());
        Ok(payload)
    }

    pub fn from_last_header(last_header: HashId) -> Self {
        Self {
            version: 70015,
            hash_count: 1,
            block_header_hashes: vec![last_header],
            stop_hash: HashId::default(),
        }
    }
}

impl Serialize for GetHeader {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let payload = self.build_payload()?;
        let message = self.build_message(GETHEADERS, Some(payload))?;
        Ok(message)
    }
}
