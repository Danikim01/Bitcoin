use super::{BlockHeader, Hashable, Serialize, InvType, Inventory, InventoryVector, Message, constants};
use std::io;

/// Struct that represents the getdata fields (https://en.bitcoin.it/wiki/Protocol_documentation#getdata)
// https://developer.bitcoin.org/reference/p2p_networking.html#getdata
#[derive(Debug, Clone)]
pub struct GetData {
    pub inventory: InventoryVector, // inv as it was received from an inv message
}

impl GetData {
    /// Create a new getdata message
    pub fn new(inventory: InventoryVector) -> Self {
        Self { inventory }
    }

    /// Create a new getdata message from a list of BlockHeaders using its hashes
    pub fn from_inv(block_headers: Vec<BlockHeader>) -> Self {
        let mut inventory_vector: Vec<Inventory> = Vec::new();
        for block_header in block_headers {
            inventory_vector.push(Inventory::new(InvType::MSGBlock, block_header.hash()));
        }
        Self::new(InventoryVector::new(inventory_vector))
    }
}

impl Serialize for GetData {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let payload = self.inventory.build_payload()?;
        let message = self.build_message(constants::commands::GETDATA, Some(payload))?;
        Ok(message)
    }

    fn deserialize(bytes: &[u8]) -> io::Result<Message> {
        match InventoryVector::deserialize(bytes)? {
            Message::Inv(inventory) => Ok(Message::GetData(Self::new(inventory))),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Expected message of type Inv"))
        }
    }
}
