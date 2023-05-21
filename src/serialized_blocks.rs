use crate::io::{self, Cursor};
use crate::merkle_tree::MerkleTree;
use crate::messages::{utility::*, BlockHeader};
use crate::raw_transaction::RawTransaction;
use bitcoin_hashes::{sha256, Hash};
use crate::io::ErrorKind::InvalidData;

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
        let serialized_block = SerializedBlock::from_bytes(&bytes).unwrap();

        let mut txid_hashes_vector = Vec::new();
        for transaction in &serialized_block.txns{
            // Serialize the transaction
            let serialized_transaction = transaction.serialize();
            // Hash the serialized transaction
            let mut transaction_hash = sha256::Hash::hash(&serialized_transaction);
            transaction_hash = sha256::Hash::hash(&transaction_hash[..]);
            txid_hashes_vector.push(transaction_hash);
        }
        
        let merkle_tree = MerkleTree::from_hashes(txid_hashes_vector);
        let merkle_tree_root_hash = merkle_tree._get_root_hash();
        println!("root:{:?}",merkle_tree_root_hash);
        match merkle_tree_root_hash{
            Some(root_hash)=>{
                println!("root hash {:?}",root_hash.to_byte_array());
                println!("root hash del header block {:?}",&serialized_block.block_header.merkle_root_hash);
            }
            None => {
                println!("Error");
            }
        }

    }
}
