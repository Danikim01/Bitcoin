use chrono::Local;

use crate::messages::{Block, Headers};
use crate::raw_transaction::RawTransaction;
use crate::utility::_encode_hex;

use super::blocks_panel::add_data_to_blocks_table;
use super::headers_panel::add_data_to_headers_table;
use super::transactions_panel::add_data_to_transactions_table;

use std::{hash, io};

/// Enum with the different tables in the interface
pub enum GtkTable {
    Transactions,
    Blocks,
    Headers,
}

/// Enum with the different types of data that can be added to a table
pub enum GtkTableData {
    /// height, date, hash, tx count (all as String)
    BlocksData(String, String, String, String),
    /// height, date, hash (all as String)
    HeadersData(String, String, String),
    /// date, hash, amount (all as String)
    TransactionData(String, String, String),
}

/// Receive a raw transaction and parse it's data to a RowData::TransactionData
pub fn table_data_from_tx(tx: &RawTransaction) -> GtkTableData {
    // need date, hash and amount
    let date = Local::now().format("%d-%m-%Y %H:%M").to_string();
    let hash_bytes = &tx.get_hash();
    let hash = _encode_hex(hash_bytes);
    let amount = format!("{:.8}", tx.get_total_output_value() as f64 / 100000000.0);

    GtkTableData::TransactionData(date, hash, amount.to_string())
}

/// Receive a block and parse it's data to a RowData::BlocksData
pub fn table_data_from_block(block: &Block) -> GtkTableData {
    // need height, date, hash and tx count

    GtkTableData::BlocksData(
        "foo".to_string(),
        "foo".to_string(),
        "foo".to_string(),
        "foo".to_string(),
    )
}

/// Receive a header and parse it's data to a RowData::HeadersData
pub fn table_data_from_headers(headers: &Headers) -> GtkTableData {
    // need height, date and hash

    GtkTableData::HeadersData("foo".to_string(), "foo".to_string(), "foo".to_string())
}

pub fn table_append_data(
    builder: gtk::Builder,
    table: GtkTable,
    data: GtkTableData,
) -> io::Result<()> {
    match table {
        GtkTable::Transactions => add_data_to_transactions_table(builder, data),
        GtkTable::Blocks => add_data_to_blocks_table(builder, data),
        GtkTable::Headers => add_data_to_headers_table(builder, data),
    }
}
