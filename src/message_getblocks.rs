use std::io::Write;
use std::net::TcpStream;
use crate::messages::Message;
use std::io::Cursor;
use std::io::Read;
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use crate::header::BlockHeader;
use crate::header::Header;

#[derive(Debug)]
pub struct GetBlocks {
    version: i32,
    hash_count: u8,
    block_header_hashes: Vec<[u8;32]>,
    stop_hash: [u8;32],
}

//Default for genesis block
impl Default for GetBlocks {
    fn default() -> Self {
        Self{
            version:70015,
            hash_count:1,
            block_header_hashes:vec![[0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63, 0xf7, 0x4f,
                0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00]], //genesis hash
            stop_hash:[0_u8; 32], //til max block hashes (500 is MAX for response)
        }

    }
}


//ver: https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
fn to_varint(value: u64) -> Vec<u8> {
    let mut buf = Vec::new();

    if value >= 0 && value <= 252{
        buf.push(value as u8);
    } else if value >= 253 && value <= 0xffff {
        buf.push(0xfd);
        buf.extend_from_slice(&(value as u16).to_le_bytes());
    } else if value >= 0x10000 && value <= 0xffffffff {
        buf.push(0xfe);
        buf.extend_from_slice(&(value as u32).to_le_bytes());
    } else if value >= 0x100000000 && value <= 0xffffffffffffffff{
        buf.push(0xff);
        buf.extend_from_slice(&(value as u64).to_le_bytes());
    }

    buf
}

fn read_i32(cursor: &mut Cursor<&[u8]>) -> Result<i32,String> {
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(i32::from_le_bytes(buf))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32,String> {
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8,String> {
    let mut buf = [0;1];
    cursor.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(u8::from_le_bytes(buf))
}

fn read_hash(cursor: &mut Cursor<&[u8]>) -> Result<[u8; 32], String> {
    let mut hash = [0u8; 32];
    cursor.read_exact(&mut hash).map_err(|e| e.to_string())?;
    Ok(hash)
}


fn read_from_varint(cursor: &mut Cursor<&[u8]>) -> Result<usize, String> {
    let first_byte = read_u8(cursor).map_err(|error| error.to_string())?;
    println!("the first byte is {}",&first_byte);
    match first_byte {
        0xff => {
            let mut buf = [0_u8; 8];
            cursor.read_exact(&mut buf).map_err(|error| error.to_string())?;
            let value = u64::from_le_bytes(buf);
            Ok(value as usize)
        }
        0xfe => {
            let mut buf = [0_u8; 4];
            cursor.read_exact(&mut buf).map_err(|error| error.to_string())?;
            let value = u32::from_le_bytes(buf);
            Ok(value as usize)
        }
        0xfd => {
            let mut buf = [0_u8; 2];
            cursor.read_exact(&mut buf).map_err(|error| error.to_string())?;
            let value = u16::from_le_bytes(buf);
            Ok(value as usize)
        }
        _ => Ok(first_byte as usize)
    }
}


impl GetBlocks {

    fn new(version:i32,hash_count:u8,block_header_hashes:Vec<[u8;32]>,stop_hash:[u8;32])->Self{
        Self{
            version,
            hash_count,
            block_header_hashes,
            stop_hash,
        }
    }

    fn build_payload(&self) ->  std::io::Result<Vec<u8>>{
        let mut payload = Vec::new();
        payload.extend(&self.version.to_le_bytes());
        let hash_count_a_enviar = to_varint(self.hash_count as u64);
        payload.extend(&hash_count_a_enviar);
        //payload.extend(&self.hash_count.to_le_bytes());
        for header_hash in &self.block_header_hashes{
            payload.extend(header_hash);
        }
        payload.extend(self.stop_hash);
        Ok(payload)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(), String> {
        let mut cursor = Cursor::new(bytes);

        // header
        let mut magic_bytes = [0_u8; 4];
        let mut command_name = [0_u8; 12];
        let mut payload_size = [0_u8; 4];
        let mut checksum = [0_u8; 4];

        // read header
        cursor
            .read_exact(&mut magic_bytes)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut command_name)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut payload_size)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut checksum)
            .map_err(|error| error.to_string())?;

        println!(
            "\nMagic bytes: {:02X?}\nCommand name: {:?}\nPayload size: {:?}\nChecksum: {:02X?}\n",
            magic_bytes,
            std::str::from_utf8(&command_name).map_err(|error| error.to_string())?,
            u32::from_le_bytes(payload_size),
            checksum
        );

        //Leo el payload
        //sabiendo que se recibe un varint        
        let mut value = read_from_varint(&mut cursor);

        println!("the value is {:?}",&value); //El value deberia ser 2000 porque se envian 32 ceros
        
        match value{
            Ok(v) => {
                //let mut headers: Vec<Header<T>> = Vec::with_capacity(v as usize);
                for _ in 0..v{
                    let version = read_i32(&mut cursor)?;
                    let prev_block_hash = read_hash(&mut cursor)?;
                    let merkle_root_hash = read_hash(&mut cursor)?;
                    let timestamp = read_u32(&mut cursor)?;
                    let nbits = read_u32(&mut cursor)?;
                    let nonce = read_u32(&mut cursor)?;
        
                    println!("Version : {}",version);
                    println!("Prev_block_hash: {:?}",prev_block_hash);
                    println!("Merkle_root_hash: {:?}",merkle_root_hash);
                    println!("Timestamp: {}",timestamp);
                    println!("nbits: {}",nbits);
                    println!("nonce {}",nonce);
        

                    
                    // headers.push(BlockHeader {
                    //     version,
                    //     prev_block_hash,
                    //     merkle_root_hash,
                    //     timestamp,
                    //     nbits,
                    //     nonce,
                    // });
                }
                Ok(())
            },
            _ => Err("El numero de headers es invalido".to_owned()),
        };
        


        // Ok(GetBlocks::new(
        //     i32::from_le_bytes(version),
        //     u8::from_le_bytes(hash_count),
        //     block_header_hashes,
        //     stop_hash
        //     ))

        Ok(())
    }
}


impl Message for GetBlocks {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let payload = self.build_payload()?;
        let message = self.build_message("getheaders".to_string(), Some(payload))?;

        stream.write_all(&message)?;
        stream.flush()?;
        Ok(())
    }
}