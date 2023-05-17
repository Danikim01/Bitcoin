use std::collections::HashMap;
use crate::block_header::BlockHeader;
use crate::messages::utility::{read_from_varint, read_hash, EndianRead};
use std::io::{self, Cursor, Error};
use std::net::TcpStream;
use crate::messages::constants::commands::HEADER;
use crate::messages::constants::header_constants::MAX_HEADER;
use crate::messages::{GetHeader, Message, MessageHeader};

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
    pub fn default() -> Self {
        Self {
            count: 0,
            block_headers: Vec::new(),
        }
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

    pub fn from_bytes(bytes: &[u8]) -> Result<Headers, io::Error> {
        let mut header = Headers::default();
        //let mut hash_headers: HashMap::<[u8; 32], BlockHeader> = HashMap::new();
        //hash_headers.insert([0_u8; 32], BlockHeader::default());
        &header.add_from_bytes(bytes);

        Ok(header)
    }

    pub fn from_stream(stream: &mut TcpStream) -> Result<Headers, Error> {
        let mut header = Headers::default();
        //let mut hash_headers: HashMap::<[u8; 32], BlockHeader> = HashMap::new();
        //hash_headers.insert([0_u8; 32], BlockHeader::default());
        &header.add_from_stream(stream);

        Ok(header)
    }

    fn add_from_bytes(&mut self, bytes: &[u8])-> Result<(), Error> {
        let mut cursor = Cursor::new(bytes);

        let count = read_from_varint(&mut cursor)?;
        println!("the headers count received is {:?}", &count); //El value deberia ser 2000 porque se envian 32 ceros

        let empty_header = BlockHeader::default();

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
                println!("prev_block_hash: {:?}", &prev_block_hash);
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

            if actual_header == empty_header{
                break;
            }
            self.block_headers.push(actual_header);
            self.count += 1;
        }

        Ok(())
    }

    fn add_from_stream(&mut self, stream: &mut TcpStream) -> Result<(), Error> {
        let headers_message = MessageHeader::read_until_command(stream, HEADER)?;

        println!(
            "Peer responded with headers message of payload size: {:?}",
            headers_message.payload_size
        );
        let data_headers = headers_message.read_payload(stream)?;
        self.add_from_bytes(&data_headers)
    }

    pub fn read_all_headers(&mut self, stream: &mut TcpStream) -> Result<(), io::Error>{

        while !self.is_last_header() {
            let getheader_message = GetHeader::from_last_header(&self.last_header_hash());
            getheader_message.send_to(stream)?;
            let headers_message = MessageHeader::read_until_command(stream, HEADER)?;

            println!(
                "Peer responded with headers message of payload size: {:?}",
                headers_message.payload_size
            );
            let data_headers = headers_message.read_payload(stream)?;
            self.add_from_bytes(&data_headers)?;
        }

        Ok(())
    }
}
