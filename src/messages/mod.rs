use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use std::io;
mod block_header;
mod block_message;
pub(crate) mod constants;
mod getdata_message;
mod getheader_message;
mod headers;
mod headers_message;
pub mod utility;
mod verack_message;
mod version_message;

pub use block_header::BlockHeader;
pub use block_message::Block;
pub use getdata_message::{GetData, InvType, Inventory};
pub use getheader_message::GetHeader;
pub use headers::MessageHeader;
pub use headers_message::Headers;
pub use verack_message::VerAck;
pub use version_message::Version;

pub type HashId = [u8; 32];

#[derive(Debug, Clone, Copy,PartialEq)]
pub struct Services {
    bitmap: u64,
}

impl Services {
    pub fn new(encoded_services: u64) -> Self {
        Self {
            bitmap: encoded_services,
        }
    }

    pub fn _is_unnamed(self) -> bool {
        self.bitmap == 0
    }

    pub fn _is_node_network(self) -> bool {
        self.bitmap & 1 != 0
    }

    pub fn _is_node_get_utxo(self) -> bool {
        self.bitmap & 2 != 0
    }

    pub fn _is_node_bloom(self) -> bool {
        self.bitmap & 4 != 0
    }

    pub fn _is_node_witness(self) -> bool {
        self.bitmap & 8 != 0
    }

    pub fn _is_node_xthin(self) -> bool {
        self.bitmap & 16 != 0
    }

    pub fn _is_node_network_limited(self) -> bool {
        self.bitmap & 1024 != 0
    }
}

impl From<[u8; 8]> for Services {
    fn from(bytes: [u8; 8]) -> Self {
        let service_code = u64::from_le_bytes(bytes);
        Services::new(service_code)
    }
}

impl From<Services> for [u8; 8] {
    fn from(value: Services) -> Self {
        value.bitmap.to_le_bytes()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Block(Block),
    GetData(GetData),
    GetHeader(GetHeader),
    Headers(Headers),
    VerAck(VerAck),
    Version(Version),
}

pub trait Hashable {
    fn hash(&self) -> HashId;
}

pub trait Serialize {
    fn serialize(&self) -> io::Result<Vec<u8>>;

    fn deserialize(bytes: &[u8]) -> Result<Message, io::Error>
    where
        Self: Sized,
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Incoming message not supported",
        ))
    }

    /// Builds message appending header with optional payload
    /// https://developer.bitcoin.org/reference/p2p_networking.html#message-headers
    fn build_message(&self, command: &str, payload: Option<Vec<u8>>) -> io::Result<Vec<u8>> {
        let magic_value: [u8; 4] = 0x0b110907u32.to_be_bytes(); // SET TO ENV
        let mut payload_size: [u8; 4] = 0_i32.to_le_bytes();

        let mut checksum: [u8; 32] = [0; 32];
        checksum[..4].copy_from_slice(&[0x5d, 0xf6, 0xe0, 0xe2]);

        if let Some(payload) = payload.as_ref() {
            payload_size = (payload.len() as u32).to_le_bytes();
            checksum = sha256::Hash::hash(payload).to_byte_array(); // first hash
            checksum = sha256::Hash::hash(&checksum).to_byte_array(); // second hash
        }

        let mut message = vec![];
        message.extend(magic_value.to_vec());
        message.extend(command.bytes());
        message.extend(payload_size.to_vec());
        message.extend(checksum[0..4].to_vec());
        if let Some(payload) = payload {
            message.extend(payload);
        }

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_single_service_from_bytes() -> Result<(), io::Error> {
        assert!(Services::from(0x00_u64.to_le_bytes())._is_unnamed());
        assert!(Services::from(0x01_u64.to_le_bytes())._is_node_network());
        assert!(Services::from(0x02_u64.to_le_bytes())._is_node_get_utxo());
        assert!(Services::from(0x04_u64.to_le_bytes())._is_node_bloom());
        assert!(Services::from(0x08_u64.to_le_bytes())._is_node_witness());
        assert!(Services::from(0x10_u64.to_le_bytes())._is_node_xthin());
        assert!(Services::from(0x0400_u64.to_le_bytes())._is_node_network_limited());
        Ok(())
    }

    #[test]
    fn test_multiple_services_from_empty_bytes() -> Result<(), io::Error> {
        let services = Services::from(0x00_u64.to_le_bytes());
        assert!(services._is_unnamed());
        assert!(!services._is_node_network());
        assert!(!services._is_node_get_utxo());
        assert!(!services._is_node_bloom());
        assert!(!services._is_node_witness());
        assert!(!services._is_node_xthin());
        assert!(!services._is_node_network_limited());
        Ok(())
    }

    #[test]
    fn test_multiple_services_from_valid_bytes() -> Result<(), io::Error> {
        let services = Services::from(0x0401_u64.to_le_bytes());
        assert!(!services._is_unnamed());
        assert!(services._is_node_network());
        assert!(!services._is_node_get_utxo());
        assert!(!services._is_node_bloom());
        assert!(!services._is_node_witness());
        assert!(!services._is_node_xthin());
        assert!(services._is_node_network_limited());
        Ok(())
    }

    #[test]
    fn test_service_into_bytes() {
        let mut bytes: [u8; 8] = Services::new(0x00_u64).into();
        assert_eq!(bytes, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Services::new(0x01_u64).into();
        assert_eq!(bytes, [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Services::new(0x02_u64).into();
        assert_eq!(bytes, [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Services::new(0x04_u64).into();
        assert_eq!(bytes, [0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Services::new(0x08_u64).into();
        assert_eq!(bytes, [0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Services::new(0x10_u64).into();
        assert_eq!(bytes, [0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Services::new(0x0400_u64).into();
        assert_eq!(bytes, [0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Services::new(0xffffffffffffffff_u64).into();
        assert_eq!(bytes, [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }
}
