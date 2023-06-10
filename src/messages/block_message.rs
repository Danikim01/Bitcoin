use crate::io::{self, Cursor};
use crate::merkle_tree::MerkleTree;
use crate::messages::{utility::*, BlockHeader, HashId, Hashable, Serialize};
use crate::raw_transaction::RawTransaction;
use crate::utility::{double_hash, encode_hex};
use crate::utxo::{Utxo, UtxoId, UtxoSet};
use bitcoin_hashes::{sha256, Hash};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::thread;

use super::Message;

#[derive(Debug, Clone)]
pub struct Block {
    pub block_header: BlockHeader,
    pub txn_count: usize,
    pub txns: Vec<RawTransaction>,
}

// https://developer.bitcoin.org/reference/block_chain.html#serialized-blocks
impl Serialize for Block {
    fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut bytes: Vec<u8> = vec![];

        let header_bytes = self.block_header.serialize();
        bytes.extend(header_bytes);
        let txn_count_bytes = to_compact_size_bytes(self.txn_count as u64);
        bytes.extend(txn_count_bytes);

        for txn in self.txns.iter() {
            let txn_bytes = txn.serialize();
            bytes.extend(txn_bytes);
        }

        Ok(bytes)
    }

    fn deserialize(bytes: &[u8]) -> Result<Message, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let block_header = BlockHeader::from_bytes(&mut cursor)?;
        let txn_count = read_from_varint(&mut cursor)?;

        let mut txns = vec![];

        let coinbase_transaction = RawTransaction::coinbase_from_bytes(&mut cursor)?;
        txns.push(coinbase_transaction);

        let other_txns = RawTransaction::vec_from_bytes(&mut cursor, txn_count as usize)?;
        txns.extend(other_txns);

        let block = Block {
            block_header,
            txn_count: txn_count as usize,
            txns,
        };
        Ok(Message::Block(block))
    }
}

impl Hashable for Block {
    fn hash(&self) -> HashId {
        self.block_header.hash()
    }
}

impl Block {
    fn hash_transactions(&self) -> Vec<sha256::Hash> {
        let mut txn_hashes: Vec<sha256::Hash> = vec![];
        self.txns.iter().for_each(|txn| {
            let txn_bytes = txn.serialize();
            let txn_hash = double_hash(&txn_bytes);
            txn_hashes.push(txn_hash);
        });
        txn_hashes
    }

    fn validate_merkle_root(&self) -> io::Result<()> {
        // hash all transactions in the block
        let txn_hashes = self.hash_transactions();

        // build merkle tree from transaction hashes
        let merkle_tree = MerkleTree::generate_from_hashes(txn_hashes); // clone txn_hashes if merkle proofing
        let root_hash = merkle_tree.get_root();

        match self.block_header.merkle_root_hash == root_hash.to_byte_array() {
            true => {
                // println!("Merkle root is valid!");
                Ok(())
            }
            false => {
                println!("\x1b[93mMerkle root is invalid!\x1b[0m");
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Merkle root hash mismatch",
                ))
            }
        }
    }

    pub fn validate(&self, utxo_set: &mut UtxoSet) -> io::Result<()> {
        let mut utxo_set_snapshot = utxo_set.clone();

        for txn in self.txns.iter() {
            txn.generate_utxo(&mut utxo_set_snapshot)?;
        }

        self.block_header.validate_proof_of_work()?;
        self.validate_merkle_root()?;

        *utxo_set = utxo_set_snapshot;

        Ok(())
    }

    pub fn from_file(file_name: String) -> io::Result<Self> {
        let path = format!("tmp/blocks/{}", file_name);
        match std::fs::read(path) {
            Ok(bytes) => {
                let message = Block::deserialize(&bytes)?;
                if let Message::Block(block) = message {
                    Ok(block)
                } else {
                    println!("found block file, but failed to deserialize");
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid block message",
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn save_to_file(&self) -> io::Result<()> {
        let file_name = format!("block_{}.dat", encode_hex(&self.hash()));
        let path = format!("tmp/blocks/{}", file_name);
        let bytes = self.serialize()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn get_days_old(&self) -> u64 {
        let current_time = Utc::now().timestamp();
        let block_time = self.block_header.timestamp as i64;
        let age = (current_time - block_time) / 86400 ;
        age as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::Block;
    use std::fs;

    #[test]
    fn test_read_serialized_block_from_bytes() -> io::Result<()> {
        // Needed to avoid github actions error
        let bytes = match fs::read("./tmp/block_message_payload.dat") {
            Ok(bytes) => bytes,
            Err(e) => {
                println!("Error reading file: {}", e);
                vec![]
            }
        };

        if !bytes.is_empty() {
            let message = Block::deserialize(&bytes).unwrap();
            let mut utxo_set: UtxoSet = HashMap::new();
            if let Message::Block(block) = message {
                block.validate(&mut utxo_set)?;
                assert_eq!(block.txn_count, block.txns.len());
            };
        }

        Ok(())
    }
}
