use std::io::{self, Cursor, Read};

#[derive(Debug)]
pub struct MessageHeader {
    pub start_string: [u8; 4],
    pub command_name: String,
    pub payload_size: u32,
    pub checksum: [u8; 4],
}

impl std::default::Default for MessageHeader {
    fn default() -> Self {
        let start_string = [0, 0, 0, 0];
        let command_name = "no_command".to_string();
        let payload_size = 0;
        let checksum = [0, 0, 0, 0];

        MessageHeader::new(start_string, command_name, payload_size, checksum)
    }
}

impl MessageHeader {
    fn new(
        start_string: [u8; 4],
        command_name: String,
        payload_size: u32,
        checksum: [u8; 4],
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
        let mut start_string = [0_u8; 4];
        let mut command_name = [0_u8; 12];
        let mut payload_size = [0_u8; 4];
        let mut checksum = [0_u8; 4];

        // read all bytes
        cursor.read_exact(&mut start_string)?;
        cursor.read_exact(&mut command_name)?;
        cursor.read_exact(&mut payload_size)?;
        cursor.read_exact(&mut checksum)?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let messageHeaderDefault = MessageHeader::default();

        assert!(messageHeaderDefault.start_string == [0, 0, 0, 0]);
        assert!("no_command"
            .to_string()
            .eq(&messageHeaderDefault.command_name));
        assert_eq!(messageHeaderDefault.payload_size, 0);
        assert!(messageHeaderDefault.checksum == [0, 0, 0, 0]);
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

        let messageHeader = MessageHeader::from_bytes(&bytes);
        println!("{:?}", messageHeader);
    }
}
