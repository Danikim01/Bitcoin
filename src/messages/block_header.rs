use crate::io::Cursor;
use crate::logger::log;
use crate::messages::{utility::*, Hashable};
use crate::utility::double_hash;
use std::io::ErrorKind::InvalidData;
use crate::messages::constants::config::{QUIET, VERBOSE};

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

    pub fn _prev_hash(&self) -> &[u8; 32] {
        &self.merkle_root_hash
    }

    pub fn serialize(&self) -> Vec<u8> {
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
        target.cmp(hash)
    }

    pub fn validate_proof_of_work(&self) -> Result<(), std::io::Error> {
        let target_threshold: [u8; 32] = Self::nbits_to_target(self.nbits);
        let block_header_hash: [u8; 32] = self.hash();
        match Self::compare_target_threshold_and_hash(&target_threshold, &block_header_hash) {
            std::cmp::Ordering::Less => {
                // The block header hash is lower than the target threshold
                log("Proof of work is valid!",VERBOSE);
            }
            std::cmp::Ordering::Greater => {
                // The block header hash is higher than the target threshold
                log("Proof of work is invalid!",QUIET);
                return Err(std::io::Error::new(InvalidData, "Invalid Proof of Work"));
            }
            std::cmp::Ordering::Equal => {
                // The block header hash is equal to the target threshold
                log("Proof of work is valid!",VERBOSE);
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
        target_arr[31 - exponent..].copy_from_slice(&target);
        target_arr
    }
}

impl Hashable for BlockHeader {
    fn hash(&self) -> [u8; 32] {
        let hash = double_hash(&self.serialize());
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&hash[..]);
        bytes
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

#[cfg(test)]
mod tests {
    use super::*;

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
        target_arr[31 - exponent..].copy_from_slice(&target);
        target_arr
    }

    #[test]
    fn test_nbits_to_target() {
        let nbits: u32 = 0x181bc330;
        let target = nbits_to_target(nbits);
        assert_eq!(
            target,
            [
                0, 0, 0, 0, 0, 0, 0, 0, 27, 195, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0
            ]
        );
    }

    #[test]
    fn test_read_block_header() {
        let block_header_bytes: [u8; 80] = [
            0, 0, 160, 32, 51, 180, 220, 237, 64, 63, 94, 99, 227, 55, 166, 166, 187, 194, 136,
            175, 122, 209, 45, 188, 74, 201, 99, 234, 23, 0, 0, 0, 0, 0, 0, 0, 219, 236, 86, 82,
            205, 174, 207, 171, 185, 174, 211, 50, 34, 116, 178, 242, 43, 7, 42, 179, 16, 189, 22,
            176, 239, 148, 154, 195, 174, 188, 14, 245, 255, 123, 51, 100, 126, 10, 41, 25, 33, 90,
            175, 108,
        ];
        let slice: &[u8] = block_header_bytes.as_ref();
        let mut cursor = Cursor::new(slice);
        let block_header = BlockHeader::from_bytes(&mut cursor).unwrap();
        assert_eq!(
            block_header.prev_block_hash,
            [
                51, 180, 220, 237, 64, 63, 94, 99, 227, 55, 166, 166, 187, 194, 136, 175, 122, 209,
                45, 188, 74, 201, 99, 234, 23, 0, 0, 0, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            block_header.merkle_root_hash,
            [
                219, 236, 86, 82, 205, 174, 207, 171, 185, 174, 211, 50, 34, 116, 178, 242, 43, 7,
                42, 179, 16, 189, 22, 176, 239, 148, 154, 195, 174, 188, 14, 245
            ]
        );
        assert_eq!(block_header.timestamp, 1681095679);
        assert_eq!(block_header.nbits, 422120062);
        assert_eq!(block_header.nonce, 1823431201);
    }
}
