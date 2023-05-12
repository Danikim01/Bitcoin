use crate::block_header::BlockHeader;
use crate::raw_transaction::RawTransaction;
#[derive(Debug)]
pub struct SerializedBlocks{
    block_header: BlockHeader,
    txn_count:usize,
    txns:RawTransaction,
}