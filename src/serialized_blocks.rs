use crate::block_header::BlockHeader;
use crate::io::{self, Cursor};
use crate::messages::utility::*;
use crate::raw_transaction::{CoinBaseInput, Outpoint, RawTransaction, TxInput, TxOutput};

#[derive(Debug)]
pub struct SerializedBlocks {
    pub block_header: BlockHeader,
    pub txn_count: usize,
    pub txns: Vec<RawTransaction>,
}

// https://developer.bitcoin.org/reference/block_chain.html#serialized-blocks
impl SerializedBlocks {
    pub fn from_bytes(bytes: &[u8]) -> Result<SerializedBlocks, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let block_header = BlockHeader::from_bytes(&mut cursor)?;
        let txn_count = read_from_varint(&mut cursor)?;

        let mut txns = vec![];

        let coinbase_transaction = RawTransaction::coinbase_from_bytes(&mut cursor)?;
        // println!("coinbase:\n{:?}", coinbase_transaction);
        txns.push(coinbase_transaction);

        let other_txns = RawTransaction::vec_from_bytes(&mut cursor, txn_count as usize)?;
        txns.extend(other_txns);

        let serialized_block = SerializedBlocks {
            block_header,
            txn_count: txn_count as usize,
            txns,
        };

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
        println!("bytes: {:?}", bytes);
        let serialized_blocks = SerializedBlocks::from_bytes(&bytes).unwrap();

        println!(
            "\n\nRead: {:?} transactions, expected: {:?}\n\n",
            serialized_blocks.txns.len(),
            serialized_blocks.txn_count
        );
    }
}
