use std::io::{self, Cursor};

use crate::messages::utility::StreamRead;
use crate::messages::{BlockHeader, Hashable, HashId, Serialize};

use super::utility::read_hash;
use super::Inventories;
use super::{constants, utility::read_from_varint, Message};

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

//ver https://en.bitcoin.it/wiki/Protocol_documentation#Inventory_Vectors
#[derive(Debug, Clone)]
pub struct Inventory {
    pub inv_type: InvType,
    pub hash: HashId,
}

impl Inventory {
    pub fn new(inv_type: InvType, hash: HashId) -> Self {
        Self { inv_type, hash }
    }

    fn to_bytes(&self) -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend(&self.inv_type.to_u32().to_le_bytes());
        // let mut hash_copy = self.hash;
        // hash_copy.reverse();
        // bytes.extend(&hash_copy);
        bytes.extend(self.hash.iter());
        Ok(bytes)
    }
}

// https://developer.bitcoin.org/reference/p2p_networking.html#getdata
#[derive(Debug, Clone)]
pub struct GetData {
    count: usize,
    inventory: Vec<Inventory>, // inv as it was received from an inv message
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

impl GetData {
    pub fn new(count: usize, inventory: Vec<Inventory>) -> Self {
        Self { count, inventory }
    }

    fn build_payload(&self) -> std::io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        let count_a_enviar = to_varint(self.count as u64);
        // println!("El count a enviar es {:?}", &self.count);
        payload.extend(&count_a_enviar);

        for inv in &self.inventory {
            let inv_a_enviar = inv.to_bytes()?;
            // println!("El inventory a enviar es {:?}", &inv_a_enviar);
            payload.extend(inv_a_enviar);
            // let mut hash_copy = inv.hash;
            // hash_copy.reverse();
            // payload.extend(inv.inv_type.to_u32().to_le_bytes());
            // payload.extend(&hash_copy);
        }
        Ok(payload)
    }

    pub fn from_inv(count: usize, block_headers: Vec<BlockHeader>) -> Self {
        let mut inventory_vector: Vec<Inventory> = Vec::new();
        for block_header in block_headers {
            inventory_vector.push(Inventory::new(InvType::MSGBlock, block_header.hash()));
        }
        Self::new(count, inventory_vector)
    }
}

impl Serialize for GetData {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let payload = self.build_payload()?;
        let message = self.build_message(constants::commands::GETDATA, Some(payload))?;
        Ok(message)
    }

    fn deserialize(bytes: &[u8]) -> Result<Message, std::io::Error> {
        let mut cursor = Cursor::new(bytes);

        let count = read_from_varint(&mut cursor)? as usize;
        let mut inventories: Inventories = Vec::new();
        for _ in 0..count {
            let inv_type = i32::from_le_stream(&mut cursor)?;
            let hash = HashId::new(read_hash(&mut cursor)?);

            inventories.push(Inventory {
                inv_type: InvType::from_u32(inv_type as u32)?,
                hash,
            })
        }

        Ok(Message::Inv(inventories))
    }
}
