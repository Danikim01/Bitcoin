use crate::io::{self, Cursor};
use crate::messages::{utility::*, BlockHeader, HashId, Hashable, Serialize};
use crate::raw_transaction::RawTransaction;

use super::Message;

#[derive(Debug)]
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

        block.block_header.validate_proof_of_work()?;
        Ok(Message::Block(block))
    }
}

impl Hashable for Block {
    fn hash(&self) -> HashId {
        self.block_header.hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_serialized_block_from_bytes() {
        let bytes = fs::read("./tmp/block_message_payload.dat").unwrap();
        Block::deserialize(&bytes).unwrap();
    }
}
