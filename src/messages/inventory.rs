use super::{constants, HashId, Serialize, Message};
use super::utility::{to_compact_size_bytes, read_from_varint, read_hash, StreamRead};
use std::io::{self, Cursor};

//https://en.bitcoin.it/wiki/Protocol_documentation#Inventory_Vectors
#[derive(Debug, Clone)]
pub struct InventoryVector {
    pub items: Vec<Inventory>,
}

impl InventoryVector {
    pub fn new(items: Vec<Inventory>) -> Self {
        Self {
            items
        }
    }

    pub fn build_payload(&self) -> std::io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        let count = to_compact_size_bytes(self.items.len() as u64);
        payload.extend(&count);

        for inv in &self.items {
            payload.extend(inv.to_bytes()?);
        }
        Ok(payload)
    }
}

impl Serialize for InventoryVector {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let payload = self.build_payload()?;
        let message = self.build_message(constants::commands::INV, Some(payload))?;
        Ok(message)
    }

    fn deserialize(bytes: &[u8]) -> Result<Message, io::Error> {
        let mut cursor = Cursor::new(bytes);
        let count = read_from_varint(&mut cursor)? as usize;
        let mut inventories: Vec<Inventory> = vec![];
        for _inventory_num in 0..count {
            inventories.push(Inventory::from_bytes(&mut cursor)?);
        }
        Ok(Message::Inv(Self::new(inventories)))
    }
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub inv_type: InvType,
    pub hash: HashId,
}

impl Inventory {
    /// Create a new inventory block
    pub fn new(inv_type: InvType, hash: HashId) -> Self {
        Self { inv_type, hash }
    }

    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.inv_type.to_u32().to_le_bytes());
        bytes.extend(self.hash.iter());
        Ok(bytes)
    }

    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let inv_type_raw = u32::from_le_stream(cursor)?;
        let inv_type = InvType::from_u32(inv_type_raw)?;
        let hash = read_hash(cursor)?;
        Ok(Self::new(inv_type, hash))
    }
}

/// All possible inventory types for the `Inv` message.
#[derive(Debug, Clone, PartialEq)]
pub enum InvType {
    _MSGError = 0,
    MSGTx = 1,
    MSGBlock = 2,
    _MSGFilteredBlock = 3,
    _MSGCompactBlock = 4,
    _MSGWitnessTx = 0x40000001,
    _MSGWitnessBlock = 0x40000002,
    _MSGFilteredWitnessBlock = 0x40000003,
}

impl InvType {
    /// Convert the inventory type to a u32 (e.g used for serialization)
    pub fn to_u32(&self) -> u32 {
        match self {
            InvType::_MSGError => 0,
            InvType::MSGTx => 1,
            InvType::MSGBlock => 2,
            InvType::_MSGFilteredBlock => 3,
            InvType::_MSGCompactBlock => 4,
            InvType::_MSGWitnessTx => 0x40000001,
            InvType::_MSGWitnessBlock => 0x40000002,
            InvType::_MSGFilteredWitnessBlock => 0x40000003,
        }
    }

    /// Convert a u32 to an inventory type (e.g used for deserialization)
    pub fn from_u32(value: u32) -> io::Result<Self> {
        match value {
            0 => Ok(InvType::_MSGError),
            1 => Ok(InvType::MSGTx),
            2 => Ok(InvType::MSGBlock),
            3 => Ok(InvType::_MSGFilteredBlock),
            4 => Ok(InvType::_MSGCompactBlock),
            0x40000001 => Ok(InvType::_MSGWitnessTx),
            0x40000002 => Ok(InvType::_MSGWitnessBlock),
            0x40000003 => Ok(InvType::_MSGFilteredWitnessBlock),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid inventory type",
            )),
        }
    }
}
