use crate::messages::Message;
use crate::messages::utility::to_varint;
use std::io::Write;
use std::net::TcpStream;

//https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
//https://developer.bitcoin.org/reference/p2p_networking.html#getheaders
#[derive(Debug)]
pub struct Headers {
    pub count: usize, //Es un Compact size uint
    pub block_headers: Vec<BlockHeader>,
}

impl Headers {
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

impl Headers {
    pub fn new(count:usize,block_headers:Vec<BlockHeader>) -> Self{
        Self{
            count,
            block_headers,
        }
    }
}

impl Headers {
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

    pub fn from_bytes(bytes: &[u8]) -> Result<Header, io::Error> {
        let mut cursor = Cursor::new(bytes);

        //Leo el payload
        //sabiendo que se recibe un varint
        let value = read_from_varint(&mut cursor)?;
        println!("the value is {:?}", &value); //El value deberia ser 2000 porque se envian 32 ceros
        let mut empty_tx = [0_u8;1];

        let mut headers: Vec<BlockHeader> = Vec::with_capacity(value as usize);
        println!("headers capacity: {}", headers.capacity());

        loop{
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

            if actual_header == BlockHeader::default(){
                break;
            }

            headers.push(actual_header);
        }

        println!("The capacity of headers is: {}", headers.len());
        Ok(Header::new(
            headers.len(),
            headers,
        ))
    }
}
