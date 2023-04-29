use std::net::TcpStream;

#[derive(Clone, Copy, Debug)]
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
    fn from(_bytes: [u8; 8]) -> Service {
        match u64::from_le_bytes(_bytes) {
            0x00 => Service::Unnamed,
            0x01 => Service::NodeNetwork,
            0x02 => Service::NodeGetUtxo,
            0x04 => Service::NodeBloom,
            0x08 => Service::NodeWitness,
            0x10 => Service::NodeXthin,
            0x0400 => Service::NodeNetworkLimited,
            _ => Service::Unrecognized,
        }
    }
}

pub trait Message {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()>;
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
}
