use crate::io::{self, Cursor};
use crate::merkle_tree::MerkleTree;
use crate::messages::{utility::*, BlockHeader, HashId, Hashable, Serialize};
use crate::raw_transaction::RawTransaction;
use crate::utxo::{Utxo, UtxoId};
use bitcoin_hashes::{sha256, Hash};
use std::collections::HashMap;

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
        Ok(vec![])
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
    pub fn validate(&self, utxo_set: &mut HashMap<UtxoId, Utxo>) -> io::Result<()> {
        let mut utxo_set_snapshot = utxo_set.clone();
        // check for double spending
        for txn in self.txns.iter() {
            txn.validate(&mut utxo_set_snapshot)?;
        }

        self.block_header.validate_proof_of_work()?;

        // check proof of inclusion -> merkle tree root
        let mut txn_hashes: Vec<sha256::Hash> = vec![];
        self.txns.iter().for_each(|txn| {
            let txn_serial = txn.serialize();
            let mut txn_hash = sha256::Hash::hash(&txn_serial);
            txn_hash = sha256::Hash::hash(&txn_hash[..]);
            txn_hashes.push(txn_hash);
        });
        // build merkle tree from transaction hashes
        let mut _merkle_tree = MerkleTree::from_hashes(txn_hashes);

        match _merkle_tree._get_root_hash() {
            Some(root_hash) if root_hash.to_byte_array() == self.block_header.merkle_root_hash => {
                utxo_set.extend(utxo_set_snapshot);
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Transactions failed proof of inclusion",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_serialized_block_from_bytes() -> io::Result<()> {
        let bytes = fs::read("./tmp/block_message_payload.dat").unwrap();
        let message = Block::deserialize(&bytes).unwrap();
        let mut utxo_set = HashMap::new();
        if let Message::Block(block) = message {
            block.validate(&mut utxo_set)?;
            assert_eq!(block.txn_count, block.txns.len());
        };
        Ok(())
    }
}
