use crate::messages::constants::commands::*;
use crate::messages::constants::header_constants::*;
use crate::messages::constants::messages::MAX_PAYLOAD_SIZE;
use std::io::{self, Cursor, Read};
use std::net::TcpStream;

#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub start_string: [u8; START_STRING_SIZE],
    pub command_name: String,
    pub payload_size: u32,
    pub checksum: [u8; CHECKSUM_SIZE],
}

impl Default for MessageHeader {
    fn default() -> Self {
        let start_string = [0, 0, 0, 0];
        let command_name = UNKNOWN.to_string();
        let payload_size = 0;
        let checksum = [0, 0, 0, 0];

        MessageHeader::new(start_string, command_name, payload_size, checksum)
    }
}

impl MessageHeader {
    fn new(
        start_string: [u8; START_STRING_SIZE],
        command_name: String,
        payload_size: u32,
        checksum: [u8; CHECKSUM_SIZE],
    ) -> Self {
        Self {
            start_string,
            command_name,
            payload_size,
            checksum,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<MessageHeader, io::Error> {
        let mut cursor = Cursor::new(bytes);

        // used bytes of each field
        let mut start_string = [0_u8; START_STRING_SIZE];
        let mut command_name = [0_u8; COMMAND_NAME_SIZE];
        let mut payload_size = [0_u8; PAYLOAD_SIZE];
        let mut checksum = [0_u8; CHECKSUM_SIZE];

        // read all bytes
        cursor.read_exact(&mut start_string)?;
        cursor.read_exact(&mut command_name)?;
        cursor.read_exact(&mut payload_size)?;
        cursor.read_exact(&mut checksum)?;

        // Ensure that command_name is a valid UTF-8 byte sequence
        if std::str::from_utf8(&command_name).is_err() {
            return Ok(Self::default());
        }

        // create MessageHeader from bytes read
        Ok(Self::new(
            start_string,
            std::str::from_utf8(&command_name)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?
                .to_string(),
            u32::from_le_bytes(payload_size),
            checksum,
        ))
    }

    pub fn from_stream(stream: &mut TcpStream) -> Result<MessageHeader, io::Error> {
        let mut header_buffer = [0_u8; HEADER_SIZE];
        let _read = stream.read(&mut header_buffer)?;
        MessageHeader::from_bytes(&header_buffer)
    }

    pub fn validate_header(&self) -> io::Result<()> {
        let commands = vec![
            GETHEADERS,
            GETDATA,
            BLOCK,
            VERSION,
            VERACK,
            HEADERS,
            UNKNOWN,
            SENDCMPCT,
            SENDHEADERS,
            PING,
            FEEFILTER,
            ADDR,
            INV,
        ];
        if commands.contains(&self.command_name.as_str()) {
            return Ok(());
        }

        let err_str = format!("Invalid command name: {}", self.command_name);
        Err(io::Error::new(io::ErrorKind::InvalidData, err_str)) // wrong error type
    }

    fn validate_payload_size(&self) -> Result<(), io::Error> {
        if self.payload_size > MAX_PAYLOAD_SIZE {
            let err_str = format!(
                "Payload size {} exceeds maximum payload size {} in command {}",
                self.payload_size, MAX_PAYLOAD_SIZE, self.command_name
            );
            println!("{}", err_str);
            // return Err(io::Error::new(
            //     io::ErrorKind::InvalidData,
            //     err_str
            // ));
        }
        Ok(())
    }

    pub fn read_payload(&self, stream: &mut TcpStream) -> Result<Vec<u8>, io::Error> {
        self.validate_payload_size()?;
        let mut payload_buffer = vec![0_u8; self.payload_size as usize];
        stream.read_exact(&mut payload_buffer)?;
        Ok(payload_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let message_header_default = MessageHeader::default();

        assert_eq!(message_header_default.start_string, [0, 0, 0, 0]);
        assert!("no_command\0\0"
            .to_string()
            .eq(&message_header_default.command_name));
        assert_eq!(message_header_default.payload_size, 0);
        assert_eq!(message_header_default.checksum, [0, 0, 0, 0]);
    }

    #[test]
    fn test_from_bytes() {
        let bytes: [u8; 24] = [
            0xf9, 0xbe, 0xb4, 0xd9, 0x76, 0x65, 0x72, 0x61, 0x63, 0x6b, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5d, 0xf6, 0xe0, 0xe2,
        ];

        // f9beb4d9 ................... Start string: Mainnet
        // 76657261636b000000000000 ... Command name: verack + null padding
        // 00000000 ................... Byte count: 0
        // 5df6e0e2 ................... Checksum: SHA256(SHA256(<empty>))

        let message_header = MessageHeader::from_bytes(&bytes);
        println!("{:?}", message_header);
    }
}
