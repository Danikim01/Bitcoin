use crate::messages::constants::commands::HEADERS;
use crate::messages::constants::header_constants::MAX_HEADER;
use crate::messages::utility::{read_from_varint, read_hash, to_varint, EndianRead};
use crate::messages::{BlockHeader, GetHeader, Message, MessageHeader, Serialize, HashId};
use crate::node::Node;
use std::fs;
use std::fs::File;
use std::collections::HashMap;
use std::io::{self, Cursor, Error, Write};
use std::net::TcpStream;

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

    pub fn trim_timestamp(&mut self, timestamp: u32) -> Result<Self, Error> {
        self.block_headers
            .retain(|header| header.timestamp > timestamp);
        self.count = self.block_headers.len();

        Ok(self.clone())
    }

    pub fn is_last_header(&self) -> bool {
        self.count % MAX_HEADER != 0
    }

    fn last_header(&self) -> &BlockHeader {
        &self.block_headers[self.block_headers.len() - 1]
    }

    pub fn last_header_hash(&self) -> HashId {
        self.last_header().hash_block_header()
    }

    pub fn into_hashmap(&self) -> HashMap<HashId, BlockHeader> {
        let mut blocks: HashMap<HashId, BlockHeader> = HashMap::new();
        for block_hdr in self.block_headers {
            blocks.insert(block_hdr.hash_block_header(), block_hdr);
        }
        blocks
    }

    pub fn from_bytes(bytes: &[u8]) -> io::Result<Headers> {
        let mut header = Headers::default();
        //let mut hash_headers: HashMap::<[u8; 32], BlockHeader> = HashMap::new();
        //hash_headers.insert([0_u8; 32], BlockHeader::default());
        header.add_from_bytes(bytes)?;

        Ok(header)
    }

    pub fn from_stream(stream: &mut TcpStream) -> Result<Headers, Error> {
        let mut header = Headers::default();
        //let mut hash_headers: HashMap::<[u8; 32], BlockHeader> = HashMap::new();
        //hash_headers.insert([0_u8; 32], BlockHeader::default());
        header.add_from_stream(stream)?;

        Ok(header)
    }

    pub fn from_file(file_name: &str) -> Result<Headers, Error> {
        let headers_bytes = fs::read(file_name)?;
        Headers::from_bytes(&headers_bytes)
    }

    fn add_from_bytes(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut temp_headers = Self::from_bytes(bytes)?;
        self.block_headers.append(&mut temp_headers.block_headers);
        self.count += temp_headers.count;
        Ok(temp_headers.count)
    }

    fn add_from_stream(&mut self, stream: &mut TcpStream) -> Result<usize, Error> {
        let headers_message = MessageHeader::read_until_command(stream, HEADERS)?;

        println!(
            "Peer responded with headers message of payload size: {:?}",
            headers_message.payload_size
        );
        let data_headers = headers_message.read_payload(stream)?;
        self.add_from_bytes(&data_headers)
    }

    pub fn read_all_headers(&mut self, node: &mut Node) -> Result<(), Error> {
        let mut headers_read = MAX_HEADER;
        while headers_read == MAX_HEADER {
            let getheader_message = GetHeader::from_last_header(self.last_header_hash());

            node.send(getheader_message.serialize()?)?;
            let headers_message = MessageHeader::read_until_command(&mut node.stream, HEADERS)?;

            let data_headers = headers_message.read_payload(&mut node.stream)?;
            headers_read = self.add_from_bytes(&data_headers)? as usize;
        }

        Ok(())
    }

    pub fn save_to_file(&self, file_name: &str) -> Result<(), Error> {
        let headers_bytes = self.serialize()?;
        let mut save_stream = File::create(file_name)?;
        save_stream.write_all(&headers_bytes)?;
        Ok(())
    }
}

impl Serialize for Headers {
    fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend(to_varint(self.count as u64));
        for header in &self.block_headers {
            bytes.extend(header.to_bytes());
            bytes.extend([0_u8; 1]);
        }
        Ok(bytes)
    }

    fn deserialize(bytes: &[u8]) -> Result<Message, io::Error> {
        let mut cursor = Cursor::new(bytes);
        let count = read_from_varint(&mut cursor)? as usize;
        let mut block_headers: Vec<BlockHeader> = vec![];
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

            block_headers.push(actual_header);
        }
        let headers = Self::new(count, block_headers);
        Ok(Message::Headers(headers))
    }
}
