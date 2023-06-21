use crate::logger::log;
use crate::messages::constants::{header_constants::MAX_HEADER, messages::GENESIS_HASHID};
use crate::messages::utility::{read_from_varint, to_varint};
use crate::messages::{BlockHeader, HashId, Hashable, Message, Serialize};
use std::fs;
use std::io::{self, Cursor};

use super::constants::config::VERBOSE;

//https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
//https://developer.bitcoin.org/reference/p2p_networking.html#getheaders
#[derive(Debug, Clone)]
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

    pub fn default() -> Self {
        Self {
            count: 0,
            block_headers: Vec::new(),
        }
    }

    pub fn trim_timestamp(&mut self, timestamp: u32) {
        self.block_headers
            .retain(|header| header.timestamp > timestamp);
        self.count = self.block_headers.len();
    }

    pub fn is_paginated(&self) -> bool {
        self.count % MAX_HEADER == 0
    }

    fn last_header(&self) -> Option<&BlockHeader> {
        if !self.block_headers.is_empty() {
            return Some(&self.block_headers[self.block_headers.len() - 1]);
        }
        None
    }

    pub fn last_header_hash(&self) -> HashId {
        return self.last_header().map_or(GENESIS_HASHID, |h| h.hash());
    }

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
        log(
            &format!("read {:?} headers from file", headers.count),
            VERBOSE,
        );
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
