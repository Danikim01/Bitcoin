use crate::io::Cursor;
use crate::messages::{utility::*, HashId, Hashable};
use crate::utility::{double_hash, to_io_err};
use std::collections::HashMap;
use std::io::{self, ErrorKind::InvalidData, Write};

/// Block header struct as defined in the Bitcoin documentation.
//https://developer.bitcoin.org/reference/block_chain.html#block-headers
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct BlockHeader {
    version: i32,
    pub prev_block_hash: HashId,
    pub next_block_hash: Option<HashId>,
    pub merkle_root_hash: HashId,
    pub timestamp: u32,
    nbits: u32,
    nonce: u32,
    pub hash: HashId,
    pub height: usize,
}

impl BlockHeader {
    pub fn new(
        version: i32,
        prev_block_hash: HashId,
        next_block_hash: Option<HashId>,
        merkle_root_hash: HashId,
        timestamp: u32,
        nbits: u32,
        nonce: u32,
    ) -> Self {
        // calculate blockHeader hash
        let mut bytes = vec![];
        bytes.extend(version.to_le_bytes());
        bytes.extend(prev_block_hash.iter());
        bytes.extend(merkle_root_hash.iter());
        bytes.extend(timestamp.to_le_bytes());
        bytes.extend(nbits.to_le_bytes());
        bytes.extend(nonce.to_le_bytes());
        let hash = double_hash(&bytes);
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&hash[..]);
        // initialize BlockHeader with its HashId
        Self {
            version,
            prev_block_hash,
            merkle_root_hash,
            next_block_hash,
            timestamp,
            nbits,
            nonce,
            hash: HashId::new(hash_bytes),
            height: 0, // block starts with height 0, changed later if prev_block_hash is found
        }
    }

    pub fn genesis(hash: HashId) -> Self {
        // return Genesis block header
        Self {
            version: 0_i32,
            prev_block_hash: HashId::default(),
            next_block_hash: None,
            merkle_root_hash: HashId::default(),
            timestamp: 0_u32,
            nbits: 0_u32,
            nonce: 0_u32,
            hash,
            height: 0,
        }
    }

    /// Create a block header from a byte array (little endian).
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
            None,
            merkle_root_hash,
            timestamp,
            nbits,
            nonce,
        );

        Ok(actual_header)
    }

    /// Deprecated
    pub fn _prev_hash(&self) -> &HashId {
        &self.merkle_root_hash
    }

    fn compare_target_threshold_and_hash(target: &HashId, hash: &HashId) -> std::cmp::Ordering {
        target.cmp(hash)
    }

    pub fn validate_proof_of_work(&self) -> Result<(), std::io::Error> {
        let target_threshold: HashId = Self::nbits_to_target(self.nbits);
        let block_header_hash: HashId = self.hash();
        match Self::compare_target_threshold_and_hash(&target_threshold, &block_header_hash) {
            std::cmp::Ordering::Less => {
                // The block header hash is lower than the target threshold
                // log("Proof of work is valid!",VERBOSE);
            }
            std::cmp::Ordering::Greater => {
                // The block header hash is higher than the target threshold
                // log("Proof of work is invalid!",QUIET);
                return Err(std::io::Error::new(InvalidData, "Invalid Proof of Work"));
            }
            std::cmp::Ordering::Equal => {
                // The block header hash is equal to the target threshold
                // log("Proof of work is valid!",VERBOSE);
            }
        }
        Ok(())
    }

    fn nbits_to_target(nbits: u32) -> HashId {
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
        HashId::new(target_arr)
    }

    /// Save the block header to a file.
    pub fn save_to_file(&self, file_name: &str) -> io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(file_name)
            .map_err(to_io_err)?;

        let mut bytes = self.serialize();
        bytes.extend([0_u8; 1]);
        file.write_all(&bytes)?;
        Ok(())
    }

    /// Serialize the block header to a byte array (little endian).
    pub fn serialize(&self) -> Vec<u8> {
        let mut header_bytes = vec![];
        header_bytes.extend(&self.version.to_le_bytes());
        header_bytes.extend(self.prev_block_hash.iter());
        header_bytes.extend(self.merkle_root_hash.iter());
        header_bytes.extend(&self.timestamp.to_le_bytes());
        header_bytes.extend(&self.nbits.to_le_bytes());
        header_bytes.extend(&self.nonce.to_le_bytes());
        header_bytes
    }

    /// Create a block header from a byte array (little endian).
    pub fn deserialize(cursor: &mut Cursor<&[u8]>) -> io::Result<BlockHeader> {
        let header = BlockHeader::from_bytes(cursor).unwrap();
        let _empty_tx = u8::from_le_stream(cursor).unwrap();
        Ok(header)
    }
}

impl Hashable for BlockHeader {
    fn hash(&self) -> HashId {
        self.hash
    }
}
#[derive(Debug, Clone)]
pub struct HeaderSet {
    headers: HashMap<HashId, BlockHeader>,
}

impl HeaderSet {
    pub fn with(hash: HashId, header: BlockHeader) -> Self {
        let mut headers = HashMap::new();
        headers.insert(hash, header);

        Self { headers }
    }

    pub fn contains_key(&self, hash: &HashId) -> bool {
        self.headers.contains_key(hash)
    }

    pub fn insert(&mut self, hash: HashId, header: BlockHeader) {
        self.headers.insert(hash, header);
    }

    pub fn entry(
        &mut self,
        hash: HashId,
    ) -> std::collections::hash_map::Entry<'_, HashId, BlockHeader> {
        self.headers.entry(hash)
    }

    pub fn get(&self, hash: &HashId) -> Option<&BlockHeader> {
        self.headers.get(hash)
    }

    pub fn get_mut(&mut self, hash: &HashId) -> Option<&mut BlockHeader> {
        self.headers.get_mut(hash)
    }

    pub fn get_next_header(&self, hash: &HashId) -> Option<&BlockHeader> {
        if let Some(header) = self.headers.get(hash) {
            if let Some(next_hash) = header.next_block_hash {
                return self.headers.get(&next_hash);
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.headers.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::messages::Block;

    use super::*;
    use std::fs;
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
    fn test_save_block_header_and_read(){
        let file_name = "test_save_block_header.dat";
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

        block_header.save_to_file(file_name).unwrap();

        //now read from file
        let bytes = fs::read(file_name).unwrap();
        let file_size = bytes.len() as u64;
        let mut cursor: Cursor<&[u8]> = Cursor::new(&bytes);
        let mut block_headers:Vec<BlockHeader> = Vec::new(); 
        while cursor.position() < file_size {
            // deserialize block_header
            let block_header = BlockHeader::deserialize(&mut cursor).unwrap();
            block_headers.push(block_header);
        }
        let block_header_from_file = block_headers[0];
        assert_eq!(
            block_header_from_file.prev_block_hash,
            HashId::new([
                51, 180, 220, 237, 64, 63, 94, 99, 227, 55, 166, 166, 187, 194, 136, 175, 122, 209,
                45, 188, 74, 201, 99, 234, 23, 0, 0, 0, 0, 0, 0, 0
            ])
        );
        assert_eq!(
            block_header_from_file.merkle_root_hash,
            HashId::new([
                219, 236, 86, 82, 205, 174, 207, 171, 185, 174, 211, 50, 34, 116, 178, 242, 43, 7,
                42, 179, 16, 189, 22, 176, 239, 148, 154, 195, 174, 188, 14, 245
            ])
        );
        assert_eq!(block_header_from_file.timestamp, 1681095679);
        assert_eq!(block_header_from_file.nbits, 422120062);
        assert_eq!(block_header_from_file.nonce, 1823431201);
        fs::remove_file(file_name).unwrap();
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
            HashId::new([
                51, 180, 220, 237, 64, 63, 94, 99, 227, 55, 166, 166, 187, 194, 136, 175, 122, 209,
                45, 188, 74, 201, 99, 234, 23, 0, 0, 0, 0, 0, 0, 0
            ])
        );
        assert_eq!(
            block_header.merkle_root_hash,
            HashId::new([
                219, 236, 86, 82, 205, 174, 207, 171, 185, 174, 211, 50, 34, 116, 178, 242, 43, 7,
                42, 179, 16, 189, 22, 176, 239, 148, 154, 195, 174, 188, 14, 245
            ])
        );
        assert_eq!(block_header.timestamp, 1681095679);
        assert_eq!(block_header.nbits, 422120062);
        assert_eq!(block_header.nonce, 1823431201);
    }

    #[test]
    fn test_push_headers_to_headerset() {
        let child_header = BlockHeader::genesis(HashId::default());
        let mut headerset = HeaderSet::with(child_header.hash, child_header);

        let parent_header = BlockHeader {
            version: 0_i32,
            prev_block_hash: HashId::default(),
            next_block_hash: Some(child_header.hash),
            merkle_root_hash: HashId::default(),
            timestamp: 0_u32,
            nbits: 0_u32,
            nonce: 0_u32,
            hash: HashId::new([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 3, 4, 5, 6, 7, 8, 9, 1,
                2, 3, 4, 5,
            ]),
            height: 0,
        };

        headerset.insert(child_header.hash, child_header);
        headerset.insert(parent_header.hash, parent_header);
        assert_eq!(
            headerset
                .headers
                .get(&parent_header.hash)
                .unwrap()
                .next_block_hash,
            Some(child_header.hash)
        );
    }
}
