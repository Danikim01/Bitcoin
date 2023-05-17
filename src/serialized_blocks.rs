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

impl SerializedBlocks {
    pub fn from_bytes(bytes: &[u8]) -> Result<(), io::Error> {
        let mut cursor = Cursor::new(bytes);

        let block_header = BlockHeader::from_bytes(&mut cursor);
        println!("{:?}", block_header);

        //txn_count
        let value = read_from_varint(&mut cursor)?;
        println!("the txn count is {:?}", &value);

        //Raw transaction
        let raw_transaction = RawTransaction::from_bytes(&mut cursor);

        Ok(())
    }
}
