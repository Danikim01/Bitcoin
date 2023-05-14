use crate::io::Cursor;
use crate::messages::utility::*;
use std::io::Read;

//https://developer.bitcoin.org/reference/block_chain.html#block-headers
#[derive(Debug, PartialEq, Clone)]
pub struct BlockHeader {
    version: i32,
    pub prev_block_hash: [u8; 32],
    merkle_root_hash: [u8; 32],
    timestamp: u32,
    nbits: u32,
    nonce: u32,
}

impl BlockHeader {
    pub fn new(
        version: i32,
        prev_block_hash: [u8; 32],
        merkle_root_hash: [u8; 32],
        timestamp: u32,
        nbits: u32,
        nonce: u32,
    ) -> Self {
        Self {
            version,
            prev_block_hash,
            merkle_root_hash,
            timestamp,
            nbits,
            nonce,
        }
    }

    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<BlockHeader, std::io::Error> {
        let mut empty_tx = [0_u8; 1];
        let version = read_i32(cursor)?;
        let prev_block_hash = read_hash(cursor)?;
        let merkle_root_hash = read_hash(cursor)?;
        let timestamp = read_u32(cursor)?;
        let nbits = read_u32(cursor)?;
        let nonce = read_u32(cursor)?;
        cursor.read_exact(&mut empty_tx)?;

        let actual_header = BlockHeader::new(
            version,
            prev_block_hash,
            merkle_root_hash,
            timestamp,
            nbits,
            nonce,
        );

        Ok(actual_header)
    }

    pub fn prev_hash(&self) -> &[u8; 32] {
        &self.merkle_root_hash
    }
}

impl std::default::Default for BlockHeader {
    fn default() -> Self {
        let version = 0_i32;
        let prev_block_hash = [0_u8; 32];
        let merkle_root_hash = [0_u8; 32];
        let timestamp = 0_u32;
        let nbits = 0_u32;
        let nonce = 0_u32;
        BlockHeader::new(
            version,
            prev_block_hash,
            merkle_root_hash,
            timestamp,
            nbits,
            nonce,
        )
    }
}
