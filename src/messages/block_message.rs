use crate::io::{self, Cursor};
use crate::messages::{utility::*, BlockHeader, HashId, Hashable, Serialize};
use crate::raw_transaction::RawTransaction;
use crate::merkle_tree::MerkleTree;
use crate::utxoset::UTXOset;
use bitcoin_hashes::{sha256, Hash};

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

        let coinbase_transaction = RawTransaction::coinbase_from_bytes(&mut cursor, utxo_set)?;
        txns.push(coinbase_transaction);

        let other_txns = RawTransaction::vec_from_bytes(&mut cursor, txn_count as usize, utxo_set)?;
        txns.extend(other_txns);

        let block = Block {
            block_header,
            txn_count: txn_count as usize,
            txns,
        };
        block.block_header.validate_proof_of_work()?;
        Ok(Message::Block(block))
    }
}

impl Hashable for Block {
    fn hash(&self) -> HashId {
        self.block_header.hash()
    }
}

impl Block {
    pub fn from_bytes(bytes: &[u8], utxo_set: &mut UTXOset) -> Result<Block, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let block_header = BlockHeader::from_bytes(&mut cursor)?;
        let txn_count = read_from_varint(&mut cursor)?;

        let mut txns = vec![];

        let coinbase_transaction = RawTransaction::coinbase_from_bytes(&mut cursor, utxo_set)?;
        txns.push(coinbase_transaction);

        let other_txns = RawTransaction::vec_from_bytes(&mut cursor, txn_count as usize, utxo_set)?;
        txns.extend(other_txns);

        let block = Block {
            block_header,
            txn_count: txn_count as usize,
            txns,
        };

        serialized_block.block_header.validate_proof_of_work()?;

        // hash all transactions
        let mut txn_hashes: Vec<sha256::Hash> = vec![];
        serialized_block.txns.iter().for_each(|txn| {
            let txn_serial = txn.serialize();
            let mut txn_hash = sha256::Hash::hash(&txn_serial);
            txn_hash = sha256::Hash::hash(&txn_hash[..]);
            txn_hashes.push(txn_hash);
        });

        // build merkle tree from transaction hashes
        let mut _merkle_tree = MerkleTree::from_hashes(txn_hashes);

        Ok(serialized_block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_serialized_block_from_bytes() {
        let bytes = fs::read("./tmp/block_message_payload.dat").unwrap();
        let mut utxo_set = UTXOset::new();
        let serialized_block = SerializedBlock::from_bytes(&bytes, &mut utxo_set).unwrap();

        assert_eq!(serialized_block.txn_count, serialized_block.txns.len());
    }
}
