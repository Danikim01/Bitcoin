use crate::io::Cursor;
use crate::messages::utility::*;
use std::io::Read;


//https://developer.bitcoin.org/reference/block_chain.html#block-headers
#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
pub struct BlockHeader {
    version:i32,
    pub prev_block_hash:[u8;32],
    merkle_root_hash:[u8;32],
    timestamp:u32,
    nbits:u32,
    nonce:u32,
}
//https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
//https://developer.bitcoin.org/reference/p2p_networking.html#getheaders
#[derive(Debug)]
pub struct Header{
    pub count:usize, //Es un Compact size uint
    pub block_headers: Vec<BlockHeader>,
}

impl Header {
    pub fn is_last_header(&self) -> bool{
        self.count < 2000
    }

    fn last_header(&self) -> &BlockHeader {
        &self.block_headers[self.block_headers.len()-1]
    }

    pub fn last_header_hash(&self) -> &[u8;32] {
        println!("Amount of headers: {}\n", self.block_headers.len());
        &self.last_header().prev_hash()
    }
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

    pub fn from_bytes(cursor:&mut Cursor<&[u8]>) -> Result<BlockHeader, std::io::Error>{

        let mut empty_tx = [0_u8;1];
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

    pub fn prev_hash(&self) -> &[u8;32]{
        &self.merkle_root_hash
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



