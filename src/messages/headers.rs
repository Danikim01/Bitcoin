use crate::block_header::BlockHeader;
use crate::messages::constants::commands::HEADER;
use crate::messages::constants::header_constants::MAX_HEADER;
use crate::messages::utility::{read_from_varint, read_hash, to_varint, EndianRead};
use crate::messages::{GetHeader, Message, MessageHeader};
use core::time;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Error, Write};
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
        self
            .block_headers
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

    pub fn last_header_hash(&self) -> [u8; 32] {
        self.last_header().hash_block_header()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Headers, Error> {
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

    fn add_from_bytes(&mut self, bytes: &[u8]) -> Result<u64, Error> {
        let mut cursor = Cursor::new(bytes);

        let count = read_from_varint(&mut cursor)?;
        for _block_num in 0..count {
            let version = i32::from_le_stream(&mut cursor)?;
            let prev_block_hash = read_hash(&mut cursor)?;
            let merkle_root_hash = read_hash(&mut cursor)?;
            let timestamp = u32::from_le_stream(&mut cursor)?;
            let nbits = u32::from_le_stream(&mut cursor)?;
            let nonce = u32::from_le_stream(&mut cursor)?;
            let _empty_tx = u8::from_le_stream(&mut cursor)?;

            /*
            if prev_block_hash == [
                0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63,
                0xf7, 0x4f, 0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00,
                0x00, 0x00, 0x00, 0x00, ] as [u8; 32]{
                println!("Funciona :D: {:?}", &prev_block_hash);
                break;
            }
            */
            let actual_header = BlockHeader::new(
                version,
                prev_block_hash,
                merkle_root_hash,
                timestamp,
                nbits,
                nonce,
            );

            self.block_headers.push(actual_header);
            self.count += 1;
        }

        Ok(count)
    }

    fn add_from_stream(&mut self, stream: &mut TcpStream) -> Result<u64, Error> {
        let headers_message = MessageHeader::read_until_command(stream, HEADER)?;

        println!(
            "Peer responded with headers message of payload size: {:?}",
            headers_message.payload_size
        );
        let data_headers = headers_message.read_payload(stream)?;
        self.add_from_bytes(&data_headers)
    }

    pub fn read_all_headers(&mut self, stream: &mut TcpStream) -> Result<(), Error> {
        let mut headers_read: u64 = MAX_HEADER as u64;
        while headers_read == MAX_HEADER as u64 {
            println!(
                "Block headers read: {:?}, requesting more starting from hash {:?}",
                self.count,
                &self.last_header_hash()
            );
            let getheader_message = GetHeader::from_last_header(&self.last_header_hash());
            getheader_message.send_to(stream)?;
            let headers_message = MessageHeader::read_until_command(stream, HEADER)?;

            println!(
                "Peer responded with headers message of payload size: {:?}",
                headers_message.payload_size
            );
            let data_headers = headers_message.read_payload(stream)?;
            headers_read = self.add_from_bytes(&data_headers)?;
        }

        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(to_varint(self.count as u64));
        for header in &self.block_headers {
            bytes.extend(header.to_bytes());
            bytes.extend([0_u8; 1]);
        }
        bytes
    }

    pub fn save_to_file(&self, file_name: &str) -> Result<(), Error> {
        let headers_bytes = self.to_bytes();
        let mut save_stream = File::create("src/headers.dat")?;
        save_stream.write_all(&headers_bytes)?;
        Ok(())
    }
}
