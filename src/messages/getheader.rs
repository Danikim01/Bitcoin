use crate::block_header::{BlockHeader, Header};
use crate::messages::utility::{read_from_varint, read_hash, to_varint, EndianRead};
use crate::messages::Message;
use std::io::{self, Cursor, Read, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub struct GetHeader {
    version: i32,
    hash_count: u8,
    block_header_hashes: Vec<[u8; 32]>,
    stop_hash: [u8; 32],
}

//default for genesis block
impl Default for GetHeader {
    fn default() -> Self {
        Self {
            version: 70015,
            hash_count: 1,
            block_header_hashes: vec![[
                0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63,
                0xf7, 0x4f, 0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ]], //genesis hash
            stop_hash: [0_u8; 32], //til max block hashes (500 is MAX for response)
        }
    }
}

impl GetHeader {
    fn new(
        version: i32,
        hash_count: u8,
        block_header_hashes: Vec<[u8; 32]>,
        stop_hash: [u8; 32],
    ) -> Self {
        Self {
            version,
            hash_count,
            block_header_hashes,
            stop_hash,
        }
    }

    fn build_payload(&self) -> std::io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        payload.extend(&self.version.to_le_bytes());
        let hash_count_a_enviar = to_varint(self.hash_count as u64);
        payload.extend(&hash_count_a_enviar);
        //payload.extend(&self.hash_count.to_le_bytes());
        for header_hash in &self.block_header_hashes {
            payload.extend(header_hash);
        }
        payload.extend(self.stop_hash);
        Ok(payload)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Header, io::Error> {
        let mut cursor = Cursor::new(bytes);

        //Leo el payload
        //sabiendo que se recibe un varint
        let value = read_from_varint(&mut cursor)?;
        println!("the value is {:?}", &value); //El value deberia ser 2000 porque se envian 32 ceros
        let mut empty_tx = [0_u8; 1];

        let mut headers: Vec<BlockHeader> = Vec::with_capacity(value as usize);
        println!("headers capacity: {}", headers.capacity());

        loop {
            let version = i32::from_le_stream(&mut cursor)?;
            let prev_block_hash = read_hash(&mut cursor)?;
            let merkle_root_hash = read_hash(&mut cursor)?;
            let timestamp = u32::from_le_stream(&mut cursor)?;
            let nbits = u32::from_le_stream(&mut cursor)?;
            let nonce = u32::from_le_stream(&mut cursor)?;
            cursor.read_exact(&mut empty_tx)?;

            let actual_header = BlockHeader::new(
                version,
                prev_block_hash,
                merkle_root_hash,
                timestamp,
                nbits,
                nonce,
            );

            if actual_header == BlockHeader::default() {
                break;
            }

            headers.push(actual_header);
        }

        println!("The capacity of headers is: {}", headers.len());
        Ok(Header::new(headers.len(), headers))
    }

    pub fn from_last_header(last_header: &[u8; 32]) -> Self {
        Self {
            version: 70015,
            hash_count: 1,
            block_header_hashes: vec![*last_header],
            stop_hash: [0; 32],
        }
    }
}

impl Message for GetHeader {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let payload = self.build_payload()?;
        let message = self.build_message("getheaders", Some(payload))?;

        stream.write_all(&message)?;
        stream.flush()?;
        Ok(())
    }
}
