use crate::message_version::Version;
use crate::messages::Message;
use std::io::Cursor;
use std::io::{Read, Write};
use std::net::TcpStream;
use bitcoin_hashes::Hash;
use bitcoin_hashes::sha256;
#[derive(Debug)]
pub struct VerAckMessage {
    pub magic: Vec<u8>,
    pub command: Vec<u8>,
    pub payload_size: u32,
    //pub payload: Vec<u8>,
    pub checksum: Vec<u8>,
}

impl VerAckMessage {
    pub fn new() -> VerAckMessage {
        VerAckMessage {
            magic: 0x0b1109079u32.to_be_bytes().to_vec(),
            command: b"verack\0\0\0\0\0\0".to_owned().to_vec(),
            ///payload: Vec::new(),
            payload_size: (0_u32),
            checksum: [0x5d, 0xf6, 0xe0, 0xe2].to_vec(),
        }
    }



    pub fn from_bytes(bytes: &[u8]) -> Result<VerAckMessage, String> {
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

        Ok(VerAckMessage{
            magic:magic_bytes.to_vec(),
            command:command_name.to_vec(),
            payload_size:payload_size[0] as u32,
            checksum:checksum.to_vec()
        })
    }
}

impl Message for VerAckMessage {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let magic = self.magic.to_owned();
        let command = self.command.to_owned();
        let payload_size = self.payload_size.to_le_bytes().to_vec();
        let checksum = self.checksum.to_vec();
        // let message = [magic, command, payload_size, checksum].concat();
        // stream.write_all(&message)?;
        println!("Envio de datos:");
        println!("Magic: {:?},command: {:?},payload_size:{:?},checksum:{:?}",&magic,&command,&payload_size,&checksum);
        stream.write(&magic);
        stream.write(&command);
        stream.write(&payload_size);
        stream.write(&checksum);
        stream.flush()?;
        Ok(())
    }
}
