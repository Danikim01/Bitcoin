//https://developer.bitcoin.org/reference/block_chain.html#block-headers
#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
pub struct BlockHeader {
    version:i32,
    prev_block_hash:[u8;32],
    merkle_root_hash:[u8;32],
    timestamp:u32,
    nbits:u32,
    nonce:u32,
}
//https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
//https://developer.bitcoin.org/reference/p2p_networking.html#getheaders
#[derive(Debug)]
pub struct Header{
    count:usize, //Es un Compact size uint
    block_headers: Vec<BlockHeader>,
}


impl BlockHeader{
    pub fn new(version:i32,prev_block_hash:[u8;32],merkle_root_hash:[u8;32],timestamp:u32,nbits:u32,nonce:u32) -> Self{
        Self{
            version,
            prev_block_hash,
            merkle_root_hash,
            timestamp,
            nbits,
            nonce
        }
    }
}

impl std::default::Default for BlockHeader {
    fn default() -> Self {
        let version = 0_i32;
        let prev_block_hash = [0_u8;32];
        let merkle_root_hash = [0_u8;32];
        let timestamp = 0_u32;
        let nbits = 0_u32;
        let nonce = 0_u32;
        BlockHeader::new(
            version,
            prev_block_hash,
            merkle_root_hash,
            timestamp,
            nbits,
            nonce
        )
    }
}

impl Header{
    pub fn new(count:usize,block_headers:Vec<BlockHeader>) -> Self{
        Self{
            count,
            block_headers,
        }
    }
}


