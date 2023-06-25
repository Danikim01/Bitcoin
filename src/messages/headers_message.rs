use crate::messages::constants::header_constants::MAX_HEADER;
use crate::messages::utility::{read_from_varint, to_varint};
use crate::messages::{BlockHeader, HashId, Hashable, Message, Serialize};
use std::fs;
use std::io::{self, Cursor};

/// Struct that contains a list of block headers and the number of headers
//https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
//https://developer.bitcoin.org/reference/p2p_networking.html#getheaders
#[derive(Debug, Clone)]
pub struct Headers {
    pub count: usize, //Es un Compact size uint
    pub block_headers: Vec<BlockHeader>,
}

impl Headers {
    /// Creates a new Headers struct
    pub fn new(count: usize, block_headers: Vec<BlockHeader>) -> Self {
        Self {
            count,
            block_headers,
        }
    }

    /// Creates a default Headers struct with an empty block_headers vector
    pub fn default() -> Self {
        Self {
            count: 0,
            block_headers: Vec::new(),
        }
    }

    /// Retains only the block headers that have a timestamp greater than the given timestamp
    pub fn trim_timestamp(&mut self, timestamp: u32) {
        self.block_headers
            .retain(|header| header.timestamp > timestamp);
        self.count = self.block_headers.len();
    }

    /// Returns true if the number of block headers is a multiple of MAX_HEADER (what means that there are more headers to download)
    pub fn is_paginated(&self) -> bool {
        self.count == MAX_HEADER
    }

    /// Doesn't check headers size, only use if you know the headers' block_headers is not empty.
    pub fn last_header_hash_unchecked(&self) -> HashId {
        return self.block_headers[self.block_headers.len() - 1].hash();
    }

    /// Returns a Headers struct with all the headers contained in the file
    pub fn from_file(file_name: &str) -> io::Result<Headers> {
        let bytes = fs::read(file_name)?;
        let file_size = bytes.len() as u64;
        let mut cursor: Cursor<&[u8]> = Cursor::new(&bytes);

        let mut headers = Headers::default();
        // while cursor has more data
        while cursor.position() < file_size {
            // deserialize block_header
            let block_header = BlockHeader::deserialize(&mut cursor)?;
            headers.count += 1;
            headers.block_headers.push(block_header);
        }
        Ok(headers)
    }
}

impl Serialize for Headers {
    fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend(to_varint(self.count as u64));
        for header in &self.block_headers {
            bytes.extend(header.serialize());
            bytes.extend([0_u8; 1]);
        }
        Ok(bytes)
    }

    fn deserialize(bytes: &[u8]) -> Result<Message, io::Error> {
        let mut cursor = Cursor::new(bytes);
        let count = read_from_varint(&mut cursor)? as usize;
        let mut block_headers: Vec<BlockHeader> = vec![];
        for _block_num in 0..count {
            block_headers.push(BlockHeader::deserialize(&mut cursor)?);
        }
        let headers = Self::new(count, block_headers);
        Ok(Message::Headers(headers))
    }
}
