use std::io::{Read, Write};

#[derive(Clone, Copy)]
pub enum Service {
    Unnamed,
    NodeNetwork,
    NodeGetUtxo,
    NodeBloom,
    NodeWitness,
    NodeXthin,
    NodeNetworkLimited,
}

impl From<[u8; 8]> for Service {
    fn from(bytes: [u8; 8]) -> Service {
        Service::Unnamed
    }
}

pub trait Message {
    fn send_to(&self, stream: &mut dyn Write) -> std::io::Result<()>;
    fn from_bytes(&self, bytes: &[u8]) -> Result<Box<dyn Message>, String>;
}
