use std::io::{Write};

#[derive(Clone, Copy, Debug)]
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
    fn from(_bytes: [u8; 8]) -> Service {
        Service::Unnamed
    }
}

pub trait Message {
    fn send_to(&self, stream: &mut dyn Write) -> std::io::Result<()>;
}
