use crate::io::Cursor;
use crate::messages::utility::*;
use bitcoin_hashes::{sha256, Hash};
use std::io::ErrorKind::InvalidData;

//https://developer.bitcoin.org/reference/block_chain.html#block-headers
#[derive(Debug, PartialEq, Clone, Copy)]
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
        let version = i32::from_le_stream(cursor)?;
        let prev_block_hash = read_hash(cursor)?;
        let merkle_root_hash = read_hash(cursor)?;
        let timestamp = u32::from_le_stream(cursor)?;
        let nbits = u32::from_le_stream(cursor)?;
        let nonce = u32::from_le_stream(cursor)?;

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

    fn compare_target_threshold_and_hash(target: &[u8; 32], hash: &[u8; 32]) -> std::cmp::Ordering {
        target.cmp(&hash)
    }

    pub fn validate_proof_of_work(&self) -> Result<(), std::io::Error> {
        let target_threshold: [u8; 32] = Self::nbits_to_target(self.nbits);
        let block_header_hash: [u8; 32] = self.hash_block_header();
        match Self::compare_target_threshold_and_hash(&target_threshold, &block_header_hash) {
            std::cmp::Ordering::Less => {
                // The block header hash is lower than the target threshold
                println!("Proof of work is valid!");
            }
            std::cmp::Ordering::Greater => {
                // The block header hash is higher than the target threshold
                println!("Proof of work is invalid!");
                return Err(std::io::Error::new(InvalidData, "Invalid Proof of Work"));
            }
            std::cmp::Ordering::Equal => {
                // The block header hash is equal to the target threshold
                println!("Proof of work is valid!");
            }
        }
        Ok(())
    }

    fn nbits_to_target(nbits: u32) -> [u8; 32] {
        let exponent = (nbits >> 24) as usize;
        let significand = nbits & 0x00FFFFFF;

        let significand_bytes = significand.to_be_bytes();
        let right_padding = vec![0u8; exponent - 3];
        let target = significand_bytes
            .into_iter()
            .chain(right_padding.into_iter())
            .collect::<Vec<u8>>();

        let mut target_arr = [0u8; 32];
        target_arr.copy_from_slice(&target);
        target_arr
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
