use crate::block_header::BlockHeader;
use crate::block_header::Header;
use crate::io::Cursor;
use crate::messages::utility::*;
use crate::messages::GetHeader;
use crate::raw_transaction::{Outpoint, RawTransaction, TxInput, TxOutput};
use ::std::io;
use std::io::Read;

#[derive(Debug)]
pub struct SerializedBlocks {
    block_header: BlockHeader,
    txn_count: usize,
    txns: RawTransaction,
}

impl SerializedBlocks {
    pub fn from_bytes(bytes: &[u8]) -> Result<(), io::Error> {
        let mut cursor = Cursor::new(bytes);

        let mut block_header = BlockHeader::from_bytes(&mut cursor);
        println!("{:?}", block_header);

        let mut block_header2 = BlockHeader::from_bytes(&mut cursor);
        println!("{:?}", block_header2);

        //txn_count
        let value = read_from_varint(&mut cursor)?;
        println!("the txn count is {:?}", &value);

        //Raw transaction
        let raw_transaction = RawTransaction::from_bytes(&mut cursor);

        Ok(())
    }
}
