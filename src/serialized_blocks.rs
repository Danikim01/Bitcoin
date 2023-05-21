use std::borrow::Borrow;

use crate::io::{self, Cursor};
use crate::messages::{utility::*, BlockHeader};
use crate::raw_transaction::RawTransaction;
use crate::merkle_tree::{MerkleTree, self};
use bitcoin_hashes::{sha256, Hash};

#[derive(Debug)]
pub struct SerializedBlock {
    pub block_header: BlockHeader,
    pub txn_count: usize,
    pub txns: Vec<RawTransaction>,
}

// https://developer.bitcoin.org/reference/block_chain.html#serialized-blocks
impl SerializedBlock {
    pub fn from_bytes(bytes: &[u8]) -> Result<SerializedBlock, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let block_header = BlockHeader::from_bytes(&mut cursor)?;
        let txn_count = read_from_varint(&mut cursor)?;

        let mut txns = vec![];

        let coinbase_transaction = RawTransaction::coinbase_from_bytes(&mut cursor)?;
        txns.push(coinbase_transaction);

        let other_txns = RawTransaction::vec_from_bytes(&mut cursor, txn_count as usize)?;
        txns.extend(other_txns);

        let serialized_block = SerializedBlock {
            block_header,
            txn_count: txn_count as usize,
            txns,
        };

        serialized_block.block_header.validate_proof_of_work()?;

        // let mut merkle_tree = MerkleTree::new();
        // for transaction in &serialized_block.txns{
        //     // Serialize the transaction
        //     let serialized_transaction = transaction.serialize();

        //     // Hash the serialized transaction
        //     let mut transaction_hash = sha256::Hash::hash(&serialized_transaction);
        //     transaction_hash = sha256::Hash::hash(&transaction_hash[..]);

        //     // Add the transaction hash to the merkle tree
        //     merkle_tree.append_txid_hash(transaction_hash);

        //     // Generate the merkle root hash
        //     merkle_tree.find_merkle_root()?;

        //     // Compare the merkle root hash to the one in the block header
        //     // TODO
        // }

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
        SerializedBlock::from_bytes(&bytes).unwrap();
    }
}
