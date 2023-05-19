use crate::block_header::BlockHeader;
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
    pub fn from_bytes(bytes: &[u8]) -> Result<(), io::Error> {
        let mut cursor = Cursor::new(bytes);

        // let block_header = BlockHeader::from_bytes(&mut cursor);
        // println!("{:?}", block_header);
        let version = read_i32(&mut cursor)?;
        let prev_block_hash = read_hash(&mut cursor)?;
        let merkle_root_hash = read_hash(&mut cursor)?;
        let timestamp = read_u32(&mut cursor)?;
        let nbits = read_u32(&mut cursor)?;
        let nonce = read_u32(&mut cursor)?;

        // let mut array = [0u8; 1];
        // cursor.read_exact(&mut array)?;

        let actual_header = BlockHeader::new(
            version,
            prev_block_hash,
            merkle_root_hash,
            timestamp,
            nbits,
            nonce,
        );
        println!("{:?}", actual_header);
        // Ok(actual_header)

        let txn_count = read_from_varint(&mut cursor)?;
        println!("the txn count is {:?}", &txn_count);

        let txns = RawTransaction::from_bytes(&mut cursor);

        Ok(())
    }
}
