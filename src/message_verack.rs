use crate::message_version::Version;
use crate::messages::Message;
use std::io::Cursor;
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub struct VerAckMessage {
    pub magic: Vec<u8>,
    pub command: Vec<u8>,
    pub payload_size: u32,
    pub payload: Vec<u8>,
    pub checksum: [u8; 4],
}

impl VerAckMessage {
    pub fn new() -> VerAckMessage {
        VerAckMessage {
            magic: 0xf9beb4d9u32.to_be_bytes().to_vec(),
            command: b"verack\0\0\0\0\0".to_owned().to_vec(),
            payload: Vec::new(),
            payload_size: (0_u32),
            checksum: [0x5d, 0xf6, 0xe0, 0xe2],
        }
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self, String> {
        let mut message = VerAckMessage::new();
        reader
            .read_exact(&mut message.magic)
            .map_err(|e| e.to_string())?;
        reader
            .read_exact(&mut message.command)
            .map_err(|e| e.to_string())?;
        reader
            .read_exact(&mut message.payload_size.to_le_bytes())
            .map_err(|e| e.to_string())?;
        reader
            .read_exact(&mut message.checksum)
            .map_err(|e| e.to_string())?;

        Ok(message)
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

        Ok(VerAckMessage::new())
    }
}

impl Message for VerAckMessage {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let message = [
            self.magic.to_owned(),
            self.command.to_owned(),
            self.payload_size.to_le_bytes().to_vec(),
            self.checksum.to_vec(),
            self.payload.to_owned(),
        ]
        .concat();

        stream.write_all(&message)?;
        stream.flush()?;
        Ok(())
    }
}
