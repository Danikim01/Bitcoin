use crate::io::{self, Cursor};
use crate::messages::{utility::*, BlockHeader};
use crate::raw_transaction::{CoinBaseInput, Outpoint, RawTransaction, TxInput, TxOutput};
use bitcoin_hashes::{sha256,Hash};

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

        let mut txid_list = Vec::new();
        for transaction in &serialized_block.txns{
            txid_list.push(MerkleTree::hash_transaction(&transaction));
        }
        println!("La lista de hashes son {:?}",txid_list);

        Ok(serialized_block)
    }
}

#[derive(Debug)]
pub struct MerkleTree{
    merkle_root:sha256::Hash,
    txid_hashes:Vec<sha256::Hash>,
}

impl MerkleTree{
    pub fn new(merkle_root:sha256::Hash,txid_hashes:Vec<sha256::Hash>) -> Self{
        Self{
            merkle_root,
            txid_hashes,
        }
    }

    pub fn hash_transaction(transaction:&RawTransaction) -> sha256::Hash {
        let first_hash = sha256::Hash::hash(&RawTransaction::serialize(&transaction));
        let second_hash = sha256::Hash::hash(&first_hash[..]);
        second_hash
    }

    pub fn find_merkle_root(transactions:Vec<RawTransaction>)-> Result<(),std::io::Error>{
        let mut txid_list = Vec::new();
        for transaction in transactions{
            txid_list.push(Self::hash_transaction(&transaction));
        }
        println!("La lista de hashes son {:?}",txid_list);
        Ok(())
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
