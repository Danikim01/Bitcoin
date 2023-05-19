use crate::block_header::{self, BlockHeader};
use crate::io::{self, Cursor};
use crate::messages::utility::*;
use crate::messages::GetHeader;
use crate::raw_transaction::{Outpoint, RawTransaction, TxInput, TxOutput};

#[derive(Debug)]
pub struct SerializedBlocks {
    block_header: BlockHeader,
    txn_count: usize,
    txns: RawTransaction,
}

// https://developer.bitcoin.org/reference/block_chain.html#serialized-blocks
impl SerializedBlocks {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let block_header = BlockHeader::from_bytes(&mut cursor)?;
        let txn_count = read_from_varint(&mut cursor)?;
        let txns = RawTransaction::from_bytes(&mut cursor)?;

        Ok(SerializedBlocks {
            block_header,
            txn_count: txn_count as usize,
            txns,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_serialized_block_from_bytes() {
        let bytes = fs::read("./src/block_message_payload.dat").unwrap();

        let serialized_blocks = SerializedBlocks::from_bytes(&bytes).unwrap();
        println!("\n{:?}", serialized_blocks);
    }
}
