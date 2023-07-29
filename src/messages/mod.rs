use crate::raw_transaction::RawTransaction;
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use std::io;
pub(crate) mod block_header;
mod block_message;
pub(crate) mod constants;
mod getdata_message;
mod getheader_message;
mod headers;
mod headers_message;
pub(crate) mod invblock_message;
pub mod merkle_tree;
mod ping_message;
pub mod utility;
mod verack_message;
pub(crate) mod version_message;

pub use block_header::BlockHeader;
pub use block_message::Block;
pub use block_message::BlockSet;
pub use getdata_message::{GetData, InvType, Inventory};
pub use getheader_message::GetHeader;
pub use headers::MessageHeader;
pub use headers_message::Headers;
pub use merkle_tree::MerkleTree;
pub use ping_message::Ping;
pub use verack_message::VerAck;
pub use version_message::Version;

/// A struct that represents a hash with 32 bytes to display in hexadecimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HashId {
    pub hash: [u8; 32],
}

impl HashId {
    pub fn new(hash: [u8; 32]) -> Self {
        Self { hash }
    }

    pub fn from_hash(hash: sha256::Hash) -> Self {
        Self::new(hash.to_byte_array())
    }

    pub fn default() -> Self {
        Self { hash: [0u8; 32] }
    }

    pub fn iter(&self) -> HashIdIter {
        HashIdIter {
            inner: self.hash.iter(),
        }
    }

    pub fn from_hex_string(hex_string: &str) -> Result<Self, io::Error> {
        let mut hash = [0u8; 32];
        if hex_string.len() != 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid hexadecimal string length. It should be 64 characters.",
            ));
        }

        let mut bytes = hex_string.as_bytes().to_owned();
        bytes.reverse();
        for (i, chunk) in bytes.chunks(2).enumerate() {
            let mut byte = <&[u8]>::clone(&chunk).to_owned();
            byte.reverse();
            let byte_str = ::std::str::from_utf8(&byte)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid UTF-8"))?;
            hash[i] = u8::from_str_radix(byte_str, 16)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid hexadecimal"))?;
        }

        Ok(HashId { hash })
    }
}

impl std::fmt::Display for HashId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.hash
                .iter()
                .rev()
                .map(|num| format!("{:02x}", num))
                .collect::<Vec<String>>()
                .join("")
        )
    }
}

impl std::str::FromStr for HashId {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        HashId::from_hex_string(s)
    }
}

pub struct HashIdIter<'a> {
    inner: std::slice::Iter<'a, u8>,
}

impl<'a> Iterator for HashIdIter<'a> {
    type Item = &'a u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

type Inventories = Vec<Inventory>;

pub enum Message {
    Block(Block),
    _GetData(GetData),
    _GetHeader(GetHeader),
    Headers(Headers),
    _VerAck(VerAck),
    Version(Version),
    Inv(Inventories),
    Transaction(RawTransaction),
    Ping(Ping),
    Ignore,
}

pub trait Hashable {
    fn hash(&self) -> HashId;
}

pub trait Serialize {
    fn serialize(&self) -> io::Result<Vec<u8>>;

    fn deserialize(_bytes: &[u8]) -> Result<Message, io::Error>
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
        let magic_value: [u8; 4] = 0x0b110907u32.to_be_bytes(); // PENDING: read from config
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
