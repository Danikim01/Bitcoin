use gtk::prelude::InetAddressExtManual;
use crate::messages::{Block, constants, Inventory, InvType, Serialize};

//ver https://en.bitcoin.it/wiki/Protocol_documentation#Inventory_Vectors
#[derive(Debug, Clone)]
pub struct InventoryBlock {
    pub inv_type: InvType,
    pub block: Block,
}

impl InventoryBlock {
    /// Create a new inventory block
    pub fn new(inv_type: InvType, block: Block) -> Self {
        Self { inv_type, block }
    }

    fn to_bytes(&self) -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        let inv_type_a_enviar = self.inv_type.to_u32();
        bytes.extend_from_slice(&inv_type_a_enviar.to_le_bytes());
        bytes.extend_from_slice(&self.block.serialize()?);
        Ok(bytes)
    }
}

fn to_varint(value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    match value {
        0..=252 => {
            buf.push(value as u8);
        }
        253..=0xffff => {
            buf.push(0xfd);
            buf.extend_from_slice(&(value as u16).to_le_bytes());
        }
        0x10000..=0xffffffff => {
            buf.push(0xfe);
            buf.extend_from_slice(&(value as u32).to_le_bytes());
        }
        _ => {
            buf.push(0xff);
            buf.extend_from_slice(&value.to_le_bytes());
        }
    }
    buf
}

//https://en.bitcoin.it/wiki/Protocol_documentation#Inventory_Vectors
#[derive(Debug, Clone)]
pub struct InventoryVector {
    count: usize,
    inventory: Vec<InventoryBlock>,
}


impl InventoryVector {
    /// Create a new inventory message
    pub fn new(count: usize, inventory: Vec<InventoryBlock>) -> Self{
        Self { count, inventory }
    }

    fn build_payload(&self) -> std::io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        let count_a_enviar = to_varint(self.count as u64);
        payload.extend(&count_a_enviar);

        for inv in &self.inventory {
            let inv_a_enviar = inv.to_bytes()?;
            payload.extend(inv_a_enviar);
        }
        Ok(payload)
    }

    /// Create a new inventory message from a list of BlockHeaders using its hashes
    pub fn from_inv(count: usize, blocks: Vec<Block>) -> Self {
        let mut inventory_vector: Vec<InventoryBlock> = Vec::new();
        for block in blocks {
            inventory_vector.push(InventoryBlock::new(InvType::MSGBlock, block));
        }
        Self::new(count, inventory_vector)
    }
}

impl Serialize for InventoryVector {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let payload = self.build_payload()?;
        let message = self.build_message(constants::commands::INV, Some(payload))?;
        Ok(message)
    }
}
