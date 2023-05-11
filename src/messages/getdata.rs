use crate::messages::Message;
use std::net::TcpStream;

// https://developer.bitcoin.org/reference/p2p_networking.html#getdata
#[derive(Debug)]
pub struct GetData {
    count: u64,
    inventory: Vec<u8> // inv as it was received from an inv message
}

impl GetData {
    fn new(
        count: u64,
        inventory: Vec<u8>,
    ) -> Self {
        Self {
            count,
            inventory,
        }
    }

    fn build_payload(&self) -> std::io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        payload.extend(&self.count.to_le_bytes());
        payload.extend(&self.inventory);
        Ok(payload)
    }

    // no need to implement from_bytes since we won't be supporting incoming getdata messages
}

impl Message for GetData {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let payload = self.build_payload()?;
        let message = self.build_message("getdata", Some(payload))?;
        stream.write_all(&message)?;
        stream.flush()?;
        Ok(())
    }
}