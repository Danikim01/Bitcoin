use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use std::net::TcpStream;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Service {
    Unnamed,
    NodeNetwork,
    NodeGetUtxo,
    NodeBloom,
    NodeWitness,
    NodeXthin,
    NodeNetworkLimited,
    Unrecognized,
}

impl From<[u8; 8]> for Service {
    fn from(bytes: [u8; 8]) -> Service {
        let service_code = u64::from_le_bytes(bytes);

        if service_code & (1 << 0) != 0 {
            return Service::NodeNetwork;
        }

        if service_code & (1 << 1) != 0 {
            return Service::NodeGetUtxo;
        }

        if service_code & (1 << 2) != 0 {
            return Service::NodeBloom;
        }

        if service_code & (1 << 3) != 0 {
            return Service::NodeWitness;
        }

        if service_code & (1 << 4) != 0 {
            return Service::NodeXthin;
        }

        if service_code & (1 << 10) != 0 {
            return Service::NodeNetworkLimited;
        }

        Service::Unrecognized
    }
}


impl Into<[u8; 8]> for Service {
    fn into(self) -> [u8; 8] {
        match self {
            Service::Unnamed => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            Service::NodeNetwork => [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            Service::NodeGetUtxo => [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            Service::NodeBloom => [0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            Service::NodeWitness => [0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            Service::NodeXthin => [0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            Service::NodeNetworkLimited => [0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            Service::Unrecognized => [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        }
    }
}

/// Returns command with zeros padded to it's right
fn get_command(cmd: String) -> [u8; 12] {
    let mut command: [u8; 12] = [0; 12];
    let bytes = cmd.as_bytes();
    command[..bytes.len()].copy_from_slice(bytes);
    command
}

pub trait Message {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()>;

    /// Builds message appending header with optional payload
    /// https://developer.bitcoin.org/reference/p2p_networking.html#message-headers
    fn build_message(&self, cmd: String, payload: Option<Vec<u8>>) -> std::io::Result<Vec<u8>> {
        let magic_value: [u8; 4] = 0x0b110907u32.to_be_bytes(); // SET TO ENV
        let command: [u8; 12] = get_command(cmd);
        let mut payload_size: [u8; 4] = 0_i32.to_le_bytes();

        let mut checksum: [u8; 32] = [0; 32];
        checksum[..4].copy_from_slice(&[0x5d, 0xf6, 0xe0, 0xe2]);

        if let Some(payload) = payload.as_ref() {
            payload_size = (payload.len() as u32).to_le_bytes();
            checksum = sha256::Hash::hash(&payload).to_byte_array(); // first hash
            checksum = sha256::Hash::hash(&checksum).to_byte_array(); // second hash
        }

        let mut message = vec![];
        message.extend(magic_value.to_vec());
        message.extend(command.to_vec());
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

    #[test]
    fn test_service_from_bytes() {
        assert!(matches!(
            Service::from(0x00_u64.to_le_bytes()),
            Service::Unnamed
        ));
        assert!(matches!(
            Service::from(0x01_u64.to_le_bytes()),
            Service::NodeNetwork
        ));
        assert!(matches!(
            Service::from(0x02_u64.to_le_bytes()),
            Service::NodeGetUtxo
        ));
        assert!(matches!(
            Service::from(0x04_u64.to_le_bytes()),
            Service::NodeBloom
        ));
        assert!(matches!(
            Service::from(0x08_u64.to_le_bytes()),
            Service::NodeWitness
        ));
        assert!(matches!(
            Service::from(0x10_u64.to_le_bytes()),
            Service::NodeXthin
        ));
        assert!(matches!(
            Service::from(0x0400_u64.to_le_bytes()),
            Service::NodeNetworkLimited
        ));
        assert!(matches!(
            Service::from(0x518_u64.to_le_bytes()),
            Service::Unrecognized
        ));
        assert!(!matches!(
            Service::from(0x00_u64.to_le_bytes()),
            Service::Unrecognized
        ));
    }

    #[test]
    fn test_service_into_bytes() {
        let mut bytes: [u8; 8] = Service::Unnamed.into();
        assert_eq!(bytes, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Service::NodeNetwork.into();
        assert_eq!(bytes, [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Service::NodeGetUtxo.into();
        assert_eq!(bytes, [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Service::NodeBloom.into();
        assert_eq!(bytes, [0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Service::NodeWitness.into();
        assert_eq!(bytes, [0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Service::NodeXthin.into();
        assert_eq!(bytes, [0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Service::NodeNetworkLimited.into();
        assert_eq!(bytes, [0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        bytes = Service::Unrecognized.into();
        assert_eq!(bytes, [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }
}
