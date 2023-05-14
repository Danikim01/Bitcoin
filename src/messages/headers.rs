use crate::block_header::BlockHeader;
use crate::messages::utility::{read_from_varint, read_hash, EndianRead};
use std::io::{self, Cursor};

//https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
//https://developer.bitcoin.org/reference/p2p_networking.html#getheaders
#[derive(Debug)]
pub struct Headers {
    pub count: usize, //Es un Compact size uint
    pub block_headers: Vec<BlockHeader>,
}

impl Headers {
    pub fn new(count: usize, block_headers: Vec<BlockHeader>) -> Self {
        Self {
            count,
            block_headers,
        }
    }

    pub fn is_last_header(&self) -> bool {
        self.count < 2000
    }

    fn last_header(&self) -> &BlockHeader {
        &self.block_headers[self.block_headers.len() - 1]
    }

    pub fn last_header_hash(&self) -> &[u8; 32] {
        &self.last_header().prev_hash()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Headers, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let count = read_from_varint(&mut cursor)?;
        println!("the headers count received is {:?}", &count); //El value deberia ser 2000 porque se envian 32 ceros

        let mut headers: Vec<BlockHeader> = Vec::with_capacity(count as usize);
        for _block_num in 0..count {
            let version = i32::from_le_stream(&mut cursor)?;
            let prev_block_hash = read_hash(&mut cursor)?;
            let merkle_root_hash = read_hash(&mut cursor)?;
            let timestamp = u32::from_le_stream(&mut cursor)?;
            let nbits = u32::from_le_stream(&mut cursor)?;
            let nonce = u32::from_le_stream(&mut cursor)?;
            let _empty_tx = u8::from_le_stream(&mut cursor)?;
            
            let actual_header = BlockHeader::new(
                version,
                prev_block_hash,
                merkle_root_hash,
                timestamp,
                nbits,
                nonce,
            );
            headers.push(actual_header);
        }

        let h = Headers::new(headers.len(), headers);
        println!("last header hash: {:?}", h.last_header_hash());
        Ok(h)
    }
}
