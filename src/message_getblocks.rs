use std::io::Write;
use std::net::TcpStream;
use crate::messages::Message;
use std::io::Cursor;
use std::io::Read;

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
        payload.extend(&self.hash_count.to_le_bytes());
        for header_hash in &self.block_header_hashes{
            payload.extend(header_hash);
        }
        payload.extend(self.stop_hash);
        Ok(payload)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<GetBlocks, String> {
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

        let mut version = [0_u8; 4];
        let mut hash_count = [0_u8; 1];
        let mut block_header = [0_u8; 32];
        let mut stop_hash =  [0_u8;32];

        cursor
            .read_exact(&mut version)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut hash_count)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut block_header)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut stop_hash)
            .map_err(|error| error.to_string())?;

        let mut block_header_hashes = Vec::new();
        block_header_hashes.push(block_header);

        Ok(GetBlocks::new(
            i32::from_le_bytes(version),
            u8::from_le_bytes(hash_count),
            block_header_hashes,
            stop_hash
            ))
    }
}


impl Message for GetBlocks {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let payload = self.build_payload()?;
        let message = self.build_message("getblocks".to_string(), Some(payload))?;

        stream.write_all(&message)?;
        stream.flush()?;
        Ok(())
    }
}