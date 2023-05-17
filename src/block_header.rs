use crate::io::Cursor;
use crate::messages::utility::*;
use std::io::Read;
use bitcoin_hashes::{sha256, Hash};

//https://developer.bitcoin.org/reference/block_chain.html#block-headers
#[derive(Debug, PartialEq, Clone)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_block_hash: [u8; 32],
    pub merkle_root_hash: [u8; 32],
    pub timestamp: u32,
    pub nbits: u32,
    pub nonce: u32,
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
        let version = read_i32(cursor)?;
        let prev_block_hash = read_hash(cursor)?;
        let merkle_root_hash = read_hash(cursor)?;
        let timestamp = read_u32(cursor)?;
        let nbits = read_u32(cursor)?;
        let nonce = read_u32(cursor)?;

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

    pub fn hash_block_header(&self) -> [u8; 32] {
        let first_hash = sha256::Hash::hash(&self.to_bytes());
        let second_hash = sha256::Hash::hash(&first_hash[..]);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&second_hash[..]);
        bytes
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut header_bytes = vec![];
        header_bytes.extend(&self.version.to_le_bytes());
        header_bytes.extend(&self.prev_block_hash);
        header_bytes.extend(&self.merkle_root_hash);
        header_bytes.extend(&self.timestamp.to_le_bytes());
        header_bytes.extend(&self.nbits.to_le_bytes());
        header_bytes.extend(&self.nonce.to_le_bytes());
        header_bytes
    }

}

impl Default for BlockHeader {
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
